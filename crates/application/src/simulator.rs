use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_bridge_protocol::{
    BRIDGE_PROTOCOL_VERSION, BridgeCapability, BridgeEnvelope, BridgeHostMessage,
    BridgeProviderMessage, ProviderConnectionState, ProviderDescriptor, ProviderManifest,
    ProviderPlatform, read_frame, valid_state_code, write_frame,
};
use wyrmgrid_domain::SimulatorTelemetrySnapshot;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);
const TELEMETRY_FREQUENCY_HZ: u8 = 1;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SimulatorBridgeError {
    #[error("That simulator provider is not installed.")]
    UnknownProvider,
    #[error("That simulator provider manifest or executable is invalid.")]
    InvalidProvider,
    #[error("That simulator provider is unavailable on this platform.")]
    ProviderUnavailable,
    #[error("Another simulator provider is already running.")]
    AnotherProviderRunning,
    #[error("That simulator provider is already running.")]
    AlreadyRunning,
    #[error("That simulator provider is not running.")]
    NotRunning,
    #[error("WyrmGrid could not start the simulator provider.")]
    LaunchFailed,
    #[error("The simulator provider did not complete its Bridge handshake.")]
    HandshakeFailed,
    #[error("The simulator provider sent an invalid or unauthorized Bridge message.")]
    ProtocolViolation,
    #[error("The simulator provider supervisor is unavailable.")]
    StateUnavailable,
}

#[derive(Debug, Clone)]
pub struct SimulatorProviderRegistration {
    manifest: ProviderManifest,
    executable: PathBuf,
}

impl SimulatorProviderRegistration {
    pub fn from_manifest_json(
        manifest_json: &str,
        executable: PathBuf,
    ) -> Result<Self, SimulatorBridgeError> {
        let manifest: ProviderManifest = serde_json::from_str(manifest_json)
            .map_err(|_| SimulatorBridgeError::InvalidProvider)?;
        manifest
            .validate()
            .map_err(|_| SimulatorBridgeError::InvalidProvider)?;
        let executable_name = executable
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(SimulatorBridgeError::InvalidProvider)?;
        let declared_name = Path::new(&manifest.entry_point)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(SimulatorBridgeError::InvalidProvider)?;
        if executable_name != declared_name {
            return Err(SimulatorBridgeError::InvalidProvider);
        }
        Ok(Self {
            manifest,
            executable,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorProviderProcessState {
    Unavailable,
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl SimulatorProviderProcessState {
    fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorProviderView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub simulators: Vec<String>,
    pub capabilities: Vec<BridgeCapability>,
    pub process_state: SimulatorProviderProcessState,
    pub connection_state: ProviderConnectionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorBridgeView {
    pub bridge_protocol_version: u32,
    pub telemetry_schema_version: u32,
    pub providers: Vec<SimulatorProviderView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_snapshot: Option<SimulatorTelemetrySnapshot>,
}

#[derive(Clone)]
pub struct SimulatorBridgeService {
    inner: Arc<SimulatorBridgeInner>,
}

struct SimulatorBridgeInner {
    registrations: BTreeMap<String, SimulatorProviderRegistration>,
    runtime: Mutex<Option<Arc<RunningProvider>>>,
}

struct RunningProvider {
    provider_id: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    process_state: Mutex<SimulatorProviderProcessState>,
    connection_state: Mutex<ProviderConnectionState>,
    descriptor: Mutex<Option<ProviderDescriptor>>,
    latest_snapshot: Mutex<Option<SimulatorTelemetrySnapshot>>,
    last_code: Mutex<Option<String>>,
    outgoing_sequence: Mutex<u64>,
}

impl SimulatorBridgeService {
    pub fn new(registrations: Vec<SimulatorProviderRegistration>) -> Self {
        let registrations = registrations
            .into_iter()
            .map(|registration| (registration.manifest.id.clone(), registration))
            .collect();
        Self {
            inner: Arc::new(SimulatorBridgeInner {
                registrations,
                runtime: Mutex::new(None),
            }),
        }
    }

    pub fn status(&self) -> Result<SimulatorBridgeView, SimulatorBridgeError> {
        let runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .clone();
        let mut providers = Vec::with_capacity(self.inner.registrations.len());
        for registration in self.inner.registrations.values() {
            let supported =
                platform_supported(&registration.manifest) && registration.executable.is_file();
            let matching_runtime = runtime
                .as_ref()
                .filter(|runtime| runtime.provider_id == registration.manifest.id);
            let (process_state, connection_state, last_code) = match matching_runtime {
                Some(runtime) => (
                    *runtime
                        .process_state
                        .lock()
                        .map_err(|_| SimulatorBridgeError::StateUnavailable)?,
                    *runtime
                        .connection_state
                        .lock()
                        .map_err(|_| SimulatorBridgeError::StateUnavailable)?,
                    runtime
                        .last_code
                        .lock()
                        .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                        .clone(),
                ),
                None if supported => (
                    SimulatorProviderProcessState::Stopped,
                    ProviderConnectionState::Stopped,
                    None,
                ),
                None => (
                    SimulatorProviderProcessState::Unavailable,
                    ProviderConnectionState::Unavailable,
                    Some("provider.executable_unavailable".into()),
                ),
            };
            providers.push(SimulatorProviderView {
                id: registration.manifest.id.clone(),
                name: registration.manifest.name.clone(),
                version: registration.manifest.version.clone(),
                simulators: registration.manifest.simulators.clone(),
                capabilities: registration.manifest.capabilities.clone(),
                process_state,
                connection_state,
                last_code,
            });
        }
        let latest_snapshot = runtime
            .as_ref()
            .map(|runtime| {
                let process_state = *runtime
                    .process_state
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                let connection_state = *runtime
                    .connection_state
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                if !snapshot_is_publishable(process_state, connection_state) {
                    return Ok(None);
                }
                runtime
                    .latest_snapshot
                    .lock()
                    .map(|snapshot| snapshot.clone())
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)
            })
            .transpose()?
            .flatten();
        Ok(SimulatorBridgeView {
            bridge_protocol_version: BRIDGE_PROTOCOL_VERSION,
            telemetry_schema_version: wyrmgrid_domain::SIMULATOR_TELEMETRY_SCHEMA_VERSION,
            providers,
            active_provider_id: runtime.as_ref().and_then(|runtime| {
                runtime
                    .process_state
                    .lock()
                    .ok()
                    .filter(|state| state.is_active())
                    .map(|_| runtime.provider_id.clone())
            }),
            latest_snapshot,
        })
    }

    pub fn latest_snapshot(
        &self,
    ) -> Result<Option<SimulatorTelemetrySnapshot>, SimulatorBridgeError> {
        let runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .clone();
        runtime
            .map(|runtime| {
                let process_state = *runtime
                    .process_state
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                let connection_state = *runtime
                    .connection_state
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                if !snapshot_is_publishable(process_state, connection_state) {
                    return Ok(None);
                }
                runtime
                    .latest_snapshot
                    .lock()
                    .map(|snapshot| snapshot.clone())
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)
            })
            .transpose()
            .map(Option::flatten)
    }

    pub fn start(&self, provider_id: &str) -> Result<SimulatorBridgeView, SimulatorBridgeError> {
        let registration = self
            .inner
            .registrations
            .get(provider_id)
            .ok_or(SimulatorBridgeError::UnknownProvider)?;
        if !platform_supported(&registration.manifest) || !registration.executable.is_file() {
            return Err(SimulatorBridgeError::ProviderUnavailable);
        }
        if !registration
            .manifest
            .capabilities
            .contains(&BridgeCapability::TelemetryRead)
        {
            return Err(SimulatorBridgeError::InvalidProvider);
        }
        if let Some(runtime) = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .as_ref()
        {
            let state = *runtime
                .process_state
                .lock()
                .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
            if state.is_active() {
                return Err(if runtime.provider_id == provider_id {
                    SimulatorBridgeError::AlreadyRunning
                } else {
                    SimulatorBridgeError::AnotherProviderRunning
                });
            }
        }

        let executable = fs::canonicalize(&registration.executable)
            .map_err(|_| SimulatorBridgeError::ProviderUnavailable)?;
        let mut child = spawn_provider(&executable)?;
        let stdin = child
            .stdin
            .take()
            .ok_or(SimulatorBridgeError::LaunchFailed)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(SimulatorBridgeError::LaunchFailed)?;
        let runtime = Arc::new(RunningProvider {
            provider_id: provider_id.into(),
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            process_state: Mutex::new(SimulatorProviderProcessState::Starting),
            connection_state: Mutex::new(ProviderConnectionState::Starting),
            descriptor: Mutex::new(None),
            latest_snapshot: Mutex::new(None),
            last_code: Mutex::new(None),
            outgoing_sequence: Mutex::new(1),
        });
        *self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? = Some(Arc::clone(&runtime));

        let requested_capabilities = vec![BridgeCapability::TelemetryRead];
        let (ready_sender, ready_receiver) = mpsc::channel();
        spawn_provider_reader(
            registration.manifest.clone(),
            requested_capabilities.clone(),
            stdout,
            Arc::clone(&runtime),
            ready_sender,
        );
        send_message(
            &runtime,
            BridgeHostMessage::Hello {
                host_version: env!("CARGO_PKG_VERSION").into(),
                provider_id: provider_id.into(),
                requested_capabilities,
            },
        )?;
        if ready_receiver.recv_timeout(STARTUP_TIMEOUT) != Ok(true) {
            fail_runtime(&runtime, "provider.handshake_failed");
            return Err(SimulatorBridgeError::HandshakeFailed);
        }
        send_message(
            &runtime,
            BridgeHostMessage::StartTelemetry {
                maximum_frequency_hz: TELEMETRY_FREQUENCY_HZ,
            },
        )?;
        *runtime
            .process_state
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? =
            SimulatorProviderProcessState::Running;
        self.status()
    }

    pub fn stop(&self, provider_id: &str) -> Result<SimulatorBridgeView, SimulatorBridgeError> {
        self.inner
            .registrations
            .get(provider_id)
            .ok_or(SimulatorBridgeError::UnknownProvider)?;
        let runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .clone()
            .filter(|runtime| runtime.provider_id == provider_id)
            .ok_or(SimulatorBridgeError::NotRunning)?;
        if !runtime
            .process_state
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .is_active()
        {
            return Err(SimulatorBridgeError::NotRunning);
        }
        *runtime
            .process_state
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? =
            SimulatorProviderProcessState::Stopping;
        let _ = send_message(&runtime, BridgeHostMessage::Shutdown);
        stop_process(&runtime)?;
        *runtime
            .process_state
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? =
            SimulatorProviderProcessState::Stopped;
        *runtime
            .connection_state
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? =
            ProviderConnectionState::Stopped;
        *runtime
            .latest_snapshot
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? = None;
        self.status()
    }
}

impl Drop for SimulatorBridgeInner {
    fn drop(&mut self) {
        if let Ok(runtime) = self.runtime.lock()
            && let Some(runtime) = runtime.as_ref()
        {
            let _ = send_message(runtime, BridgeHostMessage::Shutdown);
            let _ = stop_process(runtime);
        }
    }
}

fn platform_supported(manifest: &ProviderManifest) -> bool {
    #[cfg(all(windows, target_arch = "x86_64"))]
    let platform = ProviderPlatform::WindowsX86_64;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let platform = ProviderPlatform::LinuxX86_64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let platform = ProviderPlatform::MacosAarch64;
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let platform = ProviderPlatform::MacosX86_64;
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64")
    )))]
    return false;
    manifest.platforms.contains(&platform)
}

fn spawn_provider(executable: &Path) -> Result<Child, SimulatorBridgeError> {
    let mut command = Command::new(executable);
    command
        .current_dir(
            executable
                .parent()
                .ok_or(SimulatorBridgeError::InvalidProvider)?,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_clear();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
        for key in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        for key in ["WYRMGRID_SIMCONNECT_DLL", "MSFS2024_SDK"] {
            if let Some(value) = std::env::var_os(key)
                && Path::new(&value).is_absolute()
            {
                command.env(key, value);
            }
        }
    }
    command
        .spawn()
        .map_err(|_| SimulatorBridgeError::LaunchFailed)
}

fn send_message(
    runtime: &Arc<RunningProvider>,
    message: BridgeHostMessage,
) -> Result<(), SimulatorBridgeError> {
    let mut sequence = runtime
        .outgoing_sequence
        .lock()
        .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
    let mut stdin = runtime
        .stdin
        .lock()
        .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
    write_frame(&mut *stdin, &BridgeEnvelope::new(*sequence, message)).map_err(|_| {
        fail_runtime(runtime, "provider.write_failed");
        SimulatorBridgeError::ProtocolViolation
    })?;
    *sequence += 1;
    Ok(())
}

fn spawn_provider_reader(
    manifest: ProviderManifest,
    requested_capabilities: Vec<BridgeCapability>,
    mut stdout: std::process::ChildStdout,
    runtime: Arc<RunningProvider>,
    ready_sender: Sender<bool>,
) {
    thread::spawn(move || {
        let mut ready_sender = Some(ready_sender);
        let mut validator = ProviderMessageValidator::new(manifest, requested_capabilities);
        loop {
            let envelope: BridgeEnvelope<BridgeProviderMessage> = match read_frame(&mut stdout) {
                Ok(envelope) => envelope,
                Err(_) => {
                    let stopping = runtime
                        .process_state
                        .lock()
                        .is_ok_and(|state| *state == SimulatorProviderProcessState::Stopping);
                    if !stopping {
                        fail_runtime(&runtime, "provider.stream_closed");
                    }
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            };
            let event = match validator.apply(envelope) {
                Ok(event) => event,
                Err(_) => {
                    fail_runtime(&runtime, "provider.protocol_violation");
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            };
            match event {
                ValidatedProviderEvent::Hello(descriptor) => {
                    if let Ok(mut stored) = runtime.descriptor.lock() {
                        *stored = Some(descriptor);
                    }
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(true);
                    }
                }
                ValidatedProviderEvent::State(state, code) => {
                    if let Ok(mut connection_state) = runtime.connection_state.lock() {
                        *connection_state = state;
                    }
                    if state != ProviderConnectionState::Connected
                        && let Ok(mut latest) = runtime.latest_snapshot.lock()
                    {
                        *latest = None;
                    }
                    if let Ok(mut last_code) = runtime.last_code.lock() {
                        *last_code = Some(code);
                    }
                }
                ValidatedProviderEvent::Telemetry(snapshot) => {
                    if let Ok(mut latest) = runtime.latest_snapshot.lock() {
                        *latest = Some(snapshot);
                    }
                }
            }
        }
    });
}

struct ProviderMessageValidator {
    manifest: ProviderManifest,
    requested_capabilities: HashSet<BridgeCapability>,
    descriptor: Option<ProviderDescriptor>,
    last_envelope_sequence: u64,
    last_snapshot_sequence: u64,
}

enum ValidatedProviderEvent {
    Hello(ProviderDescriptor),
    State(ProviderConnectionState, String),
    Telemetry(SimulatorTelemetrySnapshot),
}

impl ProviderMessageValidator {
    fn new(manifest: ProviderManifest, requested_capabilities: Vec<BridgeCapability>) -> Self {
        Self {
            manifest,
            requested_capabilities: requested_capabilities.into_iter().collect(),
            descriptor: None,
            last_envelope_sequence: 0,
            last_snapshot_sequence: 0,
        }
    }

    fn apply(
        &mut self,
        envelope: BridgeEnvelope<BridgeProviderMessage>,
    ) -> Result<ValidatedProviderEvent, SimulatorBridgeError> {
        envelope
            .validate_header()
            .map_err(|_| SimulatorBridgeError::ProtocolViolation)?;
        if envelope.sequence <= self.last_envelope_sequence {
            return Err(SimulatorBridgeError::ProtocolViolation);
        }
        self.last_envelope_sequence = envelope.sequence;
        match envelope.payload {
            BridgeProviderMessage::Hello { provider }
                if self.descriptor.is_none()
                    && provider.validate()
                    && provider.id == self.manifest.id
                    && provider.version == self.manifest.version
                    && self.manifest.simulators.contains(&provider.simulator)
                    && provider
                        .capabilities
                        .iter()
                        .all(|capability| self.manifest.capabilities.contains(capability))
                    && self
                        .requested_capabilities
                        .iter()
                        .all(|capability| provider.capabilities.contains(capability)) =>
            {
                self.descriptor = Some(provider.clone());
                Ok(ValidatedProviderEvent::Hello(provider))
            }
            BridgeProviderMessage::State { state, code }
                if self.descriptor.is_some() && valid_state_code(&code) =>
            {
                Ok(ValidatedProviderEvent::State(state, code))
            }
            BridgeProviderMessage::Telemetry { snapshot }
                if self.descriptor.is_some()
                    && self
                        .requested_capabilities
                        .contains(&BridgeCapability::TelemetryRead)
                    && snapshot.sequence > self.last_snapshot_sequence
                    && snapshot.validate().is_ok()
                    && snapshot.provenance.provider == self.manifest.id
                    && self
                        .manifest
                        .simulators
                        .contains(&snapshot.simulator.family) =>
            {
                self.last_snapshot_sequence = snapshot.sequence;
                Ok(ValidatedProviderEvent::Telemetry(snapshot))
            }
            _ => Err(SimulatorBridgeError::ProtocolViolation),
        }
    }
}

fn fail_runtime(runtime: &Arc<RunningProvider>, code: &str) {
    if let Ok(mut state) = runtime.process_state.lock() {
        *state = SimulatorProviderProcessState::Failed;
    }
    if let Ok(mut connection_state) = runtime.connection_state.lock() {
        *connection_state = ProviderConnectionState::Failed;
    }
    if let Ok(mut last_code) = runtime.last_code.lock() {
        *last_code = Some(code.into());
    }
    if let Ok(mut latest) = runtime.latest_snapshot.lock() {
        *latest = None;
    }
    if let Ok(mut child) = runtime.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn snapshot_is_publishable(
    process_state: SimulatorProviderProcessState,
    connection_state: ProviderConnectionState,
) -> bool {
    process_state == SimulatorProviderProcessState::Running
        && connection_state == ProviderConnectionState::Connected
}

fn stop_process(runtime: &Arc<RunningProvider>) -> Result<(), SimulatorBridgeError> {
    let mut child = runtime
        .child
        .lock()
        .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if started.elapsed() < SHUTDOWN_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => {
                child
                    .kill()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                child
                    .wait()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/simulator.rs"]
mod tests;

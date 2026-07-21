use crate::{
    ExtensionPackageManagementError, ExtensionPackageService, ManagedSimulatorProviderPackageView,
    SimulatorProviderPackageInspection, inspect_simulator_provider_package,
};
use serde::{Deserialize, Serialize};
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
use wyrmgrid_storage::{SimulatorPreferencesRecord, Store};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);
const TELEMETRY_FREQUENCY_HZ: u8 = 1;
const TELEMETRY_STALE_AFTER: Duration = Duration::from_secs(5);

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
    #[error("WyrmGrid could not read or save its local simulator preferences.")]
    PreferencesUnavailable,
    #[error("Choose an installed simulator provider before enabling automatic start.")]
    InvalidPreferences,
    #[error("The selected simulator provider package is invalid or unsupported.")]
    InvalidPackage,
    #[error("Local simulator provider package storage is unavailable.")]
    PackageStorageUnavailable,
    #[error("That simulator provider version already exists with different package contents.")]
    PackageVersionConflict,
    #[error("Stop this simulator provider before changing its installed package.")]
    PackageInUse,
    #[error("No previous simulator provider version is available for rollback.")]
    RollbackUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatorPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_provider_id: Option<String>,
    pub start_with_wyrmgrid: bool,
}

pub trait SimulatorPreferencesRepository: Send + Sync + 'static {
    fn load_simulator_preferences(
        &self,
    ) -> Result<Option<SimulatorPreferences>, SimulatorBridgeError>;
    fn save_simulator_preferences(
        &self,
        preferences: &SimulatorPreferences,
    ) -> Result<(), SimulatorBridgeError>;
}

impl SimulatorPreferencesRepository for Store {
    fn load_simulator_preferences(
        &self,
    ) -> Result<Option<SimulatorPreferences>, SimulatorBridgeError> {
        self.load_simulator_preferences_record()
            .map(|value| value.map(preferences_from_record))
            .map_err(|_| SimulatorBridgeError::PreferencesUnavailable)
    }

    fn save_simulator_preferences(
        &self,
        preferences: &SimulatorPreferences,
    ) -> Result<(), SimulatorBridgeError> {
        self.save_simulator_preferences_record(&SimulatorPreferencesRecord {
            selected_provider_id: preferences.selected_provider_id.clone(),
            start_with_wyrmgrid: preferences.start_with_wyrmgrid,
        })
        .map_err(|_| SimulatorBridgeError::PreferencesUnavailable)
    }
}

pub struct SimulatorSettingsService<R> {
    repository: R,
    providers: SimulatorProviderIds,
}

enum SimulatorProviderIds {
    Static(Vec<String>),
    Bridge(SimulatorBridgeService),
}

impl<R: SimulatorPreferencesRepository> SimulatorSettingsService<R> {
    pub fn new(repository: R, provider_ids: Vec<String>) -> Self {
        Self {
            repository,
            providers: SimulatorProviderIds::Static(provider_ids),
        }
    }

    pub fn with_bridge(repository: R, bridge: SimulatorBridgeService) -> Self {
        Self {
            repository,
            providers: SimulatorProviderIds::Bridge(bridge),
        }
    }

    pub fn status(&self) -> Result<SimulatorPreferences, SimulatorBridgeError> {
        let stored = self.repository.load_simulator_preferences()?;
        let provider_ids = self.provider_ids()?;
        Ok(match stored {
            Some(preferences) if self.valid(&preferences, &provider_ids) => preferences,
            _ => self.defaults(&provider_ids),
        })
    }

    pub fn update(
        &self,
        preferences: SimulatorPreferences,
    ) -> Result<SimulatorPreferences, SimulatorBridgeError> {
        let provider_ids = self.provider_ids()?;
        if !self.valid(&preferences, &provider_ids) {
            return Err(SimulatorBridgeError::InvalidPreferences);
        }
        self.repository.save_simulator_preferences(&preferences)?;
        Ok(preferences)
    }

    pub fn select_provider(
        &self,
        provider_id: &str,
    ) -> Result<SimulatorPreferences, SimulatorBridgeError> {
        if !self
            .provider_ids()?
            .iter()
            .any(|known| known == provider_id)
        {
            return Err(SimulatorBridgeError::UnknownProvider);
        }
        let mut preferences = self.status()?;
        preferences.selected_provider_id = Some(provider_id.to_owned());
        self.update(preferences)
    }

    pub fn startup_provider_id(&self) -> Result<Option<String>, SimulatorBridgeError> {
        let preferences = self.status()?;
        Ok(preferences
            .start_with_wyrmgrid
            .then_some(preferences.selected_provider_id)
            .flatten())
    }

    pub fn remove_managed_provider(&self, provider_id: &str) -> Result<(), SimulatorBridgeError> {
        let SimulatorProviderIds::Bridge(bridge) = &self.providers else {
            return Err(SimulatorBridgeError::PackageStorageUnavailable);
        };
        bridge.ensure_provider_inactive(provider_id)?;
        if self
            .repository
            .load_simulator_preferences()?
            .is_some_and(|preferences| {
                preferences.selected_provider_id.as_deref() == Some(provider_id)
            })
        {
            self.repository
                .save_simulator_preferences(&SimulatorPreferences {
                    selected_provider_id: None,
                    start_with_wyrmgrid: false,
                })?;
        }
        bridge.remove_managed_provider(provider_id)
    }

    pub fn set_managed_provider_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedSimulatorProviderPackageView, SimulatorBridgeError> {
        let SimulatorProviderIds::Bridge(bridge) = &self.providers else {
            return Err(SimulatorBridgeError::PackageStorageUnavailable);
        };
        bridge.ensure_provider_inactive(provider_id)?;
        if !enabled
            && let Some(mut preferences) = self.repository.load_simulator_preferences()?
            && preferences.selected_provider_id.as_deref() == Some(provider_id)
            && preferences.start_with_wyrmgrid
        {
            preferences.start_with_wyrmgrid = false;
            self.repository.save_simulator_preferences(&preferences)?;
        }
        bridge.set_managed_provider_enabled(provider_id, enabled)
    }

    fn defaults(&self, provider_ids: &[String]) -> SimulatorPreferences {
        SimulatorPreferences {
            selected_provider_id: provider_ids.first().cloned(),
            start_with_wyrmgrid: false,
        }
    }

    fn valid(&self, preferences: &SimulatorPreferences, provider_ids: &[String]) -> bool {
        let selected_is_known = preferences
            .selected_provider_id
            .as_ref()
            .is_none_or(|selected| provider_ids.iter().any(|known| known == selected));
        selected_is_known
            && (!preferences.start_with_wyrmgrid || preferences.selected_provider_id.is_some())
    }

    fn provider_ids(&self) -> Result<Vec<String>, SimulatorBridgeError> {
        match &self.providers {
            SimulatorProviderIds::Static(provider_ids) => Ok(provider_ids.clone()),
            SimulatorProviderIds::Bridge(bridge) => bridge.provider_ids_checked(),
        }
    }
}

fn preferences_from_record(record: SimulatorPreferencesRecord) -> SimulatorPreferences {
    SimulatorPreferences {
        selected_provider_id: record.selected_provider_id,
        start_with_wyrmgrid: record.start_with_wyrmgrid,
    }
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

    fn from_managed_package(
        manifest: ProviderManifest,
        package_root: PathBuf,
    ) -> Result<Self, SimulatorBridgeError> {
        manifest
            .validate()
            .map_err(|_| SimulatorBridgeError::InvalidProvider)?;
        let executable = package_root.join(&manifest.entry_point);
        if !executable.is_file() {
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
    pub telemetry_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_snapshot_age_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_age_seconds: Option<u64>,
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

pub trait SimulatorTelemetryObserver: Send + Sync + 'static {
    fn observe(&self, provider_id: &str, snapshot: &SimulatorTelemetrySnapshot);
}

#[derive(Clone)]
pub struct SimulatorBridgeService {
    inner: Arc<SimulatorBridgeInner>,
}

struct SimulatorBridgeInner {
    registrations: BTreeMap<String, SimulatorProviderRegistration>,
    packages: Option<ExtensionPackageService>,
    runtime: Mutex<Option<Arc<RunningProvider>>>,
    telemetry_observer: Option<Arc<dyn SimulatorTelemetryObserver>>,
}

struct RunningProvider {
    provider_id: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    process_state: Mutex<SimulatorProviderProcessState>,
    connection_state: Mutex<ProviderConnectionState>,
    descriptor: Mutex<Option<ProviderDescriptor>>,
    latest_snapshot: Mutex<Option<SimulatorTelemetrySnapshot>>,
    latest_snapshot_at: Mutex<Option<Instant>>,
    connected_at: Mutex<Option<Instant>>,
    last_code: Mutex<Option<String>>,
    outgoing_sequence: Mutex<u64>,
}

impl SimulatorBridgeService {
    pub fn new(registrations: Vec<SimulatorProviderRegistration>) -> Self {
        Self::with_telemetry_observer(registrations, None)
    }

    pub fn with_telemetry_observer(
        registrations: Vec<SimulatorProviderRegistration>,
        telemetry_observer: Option<Arc<dyn SimulatorTelemetryObserver>>,
    ) -> Self {
        Self::with_optional_extension_packages(registrations, None, telemetry_observer)
    }

    pub fn with_extension_packages(
        registrations: Vec<SimulatorProviderRegistration>,
        packages: ExtensionPackageService,
        telemetry_observer: Option<Arc<dyn SimulatorTelemetryObserver>>,
    ) -> Self {
        Self::with_optional_extension_packages(registrations, Some(packages), telemetry_observer)
    }

    fn with_optional_extension_packages(
        registrations: Vec<SimulatorProviderRegistration>,
        packages: Option<ExtensionPackageService>,
        telemetry_observer: Option<Arc<dyn SimulatorTelemetryObserver>>,
    ) -> Self {
        let registrations = registrations
            .into_iter()
            .map(|registration| (registration.manifest.id.clone(), registration))
            .collect();
        Self {
            inner: Arc::new(SimulatorBridgeInner {
                registrations,
                packages,
                runtime: Mutex::new(None),
                telemetry_observer,
            }),
        }
    }

    pub fn status(&self) -> Result<SimulatorBridgeView, SimulatorBridgeError> {
        let registrations = self.registrations()?;
        let runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
            .clone();
        let mut providers = Vec::with_capacity(registrations.len());
        for registration in registrations.values() {
            let supported =
                platform_supported(&registration.manifest) && registration.executable.is_file();
            let matching_runtime = runtime
                .as_ref()
                .filter(|runtime| runtime.provider_id == registration.manifest.id);
            let (process_state, connection_state, last_code, snapshot_age, connected_age) =
                match matching_runtime {
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
                        runtime
                            .latest_snapshot_at
                            .lock()
                            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                            .map(|received| received.elapsed()),
                        runtime
                            .connected_at
                            .lock()
                            .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                            .map(|connected| connected.elapsed()),
                    ),
                    None if supported => (
                        SimulatorProviderProcessState::Stopped,
                        ProviderConnectionState::Stopped,
                        None,
                        None,
                        None,
                    ),
                    None => (
                        SimulatorProviderProcessState::Unavailable,
                        ProviderConnectionState::Unavailable,
                        Some("provider.executable_unavailable".into()),
                        None,
                        None,
                    ),
                };
            let telemetry_stale = telemetry_is_stale(connection_state, snapshot_age, connected_age);
            providers.push(SimulatorProviderView {
                id: registration.manifest.id.clone(),
                name: registration.manifest.name.clone(),
                version: registration.manifest.version.clone(),
                simulators: registration.manifest.simulators.clone(),
                capabilities: registration.manifest.capabilities.clone(),
                process_state,
                connection_state,
                last_code,
                telemetry_stale,
                latest_snapshot_age_seconds: snapshot_age.map(|age| age.as_secs()),
                connected_age_seconds: connected_age.map(|age| age.as_secs()),
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
                let snapshot_age = runtime
                    .latest_snapshot_at
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                    .map(|received| received.elapsed());
                let connected_age = runtime
                    .connected_at
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                    .map(|connected| connected.elapsed());
                if !snapshot_is_publishable(process_state, connection_state)
                    || telemetry_is_stale(connection_state, snapshot_age, connected_age)
                {
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
                let snapshot_age = runtime
                    .latest_snapshot_at
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                    .map(|received| received.elapsed());
                let connected_age = runtime
                    .connected_at
                    .lock()
                    .map_err(|_| SimulatorBridgeError::StateUnavailable)?
                    .map(|connected| connected.elapsed());
                if !snapshot_is_publishable(process_state, connection_state)
                    || telemetry_is_stale(connection_state, snapshot_age, connected_age)
                {
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
        let registrations = self.registrations()?;
        let registration = registrations
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
            latest_snapshot_at: Mutex::new(None),
            connected_at: Mutex::new(None),
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
            self.inner.telemetry_observer.clone(),
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

    pub fn provider_ids(&self) -> Vec<String> {
        self.provider_ids_checked().unwrap_or_default()
    }

    pub fn inspect_provider_package(
        &self,
        path: &Path,
    ) -> Result<SimulatorProviderPackageInspection, SimulatorBridgeError> {
        inspect_simulator_provider_package(path).map_err(|_| SimulatorBridgeError::InvalidPackage)
    }

    pub fn list_managed_provider_packages(
        &self,
    ) -> Result<Vec<ManagedSimulatorProviderPackageView>, SimulatorBridgeError> {
        self.packages()?
            .list_simulator_provider_packages()
            .map_err(simulator_package_error)
    }

    pub fn seed_first_party_provider_package(
        &self,
        path: &Path,
    ) -> Result<(), SimulatorBridgeError> {
        self.packages()?
            .seed_first_party_simulator_provider_package(path)
            .map(|_| ())
            .map_err(simulator_package_error)
    }

    pub fn install_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedSimulatorProviderPackageView, SimulatorBridgeError> {
        let inspection = self.inspect_provider_package(path)?;
        self.ensure_provider_inactive(&inspection.id)?;
        self.packages()?
            .install_simulator_provider_package(path)
            .map_err(simulator_package_error)
    }

    pub fn set_managed_provider_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedSimulatorProviderPackageView, SimulatorBridgeError> {
        self.ensure_provider_inactive(provider_id)?;
        self.packages()?
            .set_simulator_provider_enabled(provider_id, enabled)
            .map_err(simulator_package_error)
    }

    pub fn rollback_managed_provider(
        &self,
        provider_id: &str,
    ) -> Result<ManagedSimulatorProviderPackageView, SimulatorBridgeError> {
        self.ensure_provider_inactive(provider_id)?;
        self.packages()?
            .rollback_simulator_provider(provider_id)
            .map_err(simulator_package_error)
    }

    pub fn remove_managed_provider(&self, provider_id: &str) -> Result<(), SimulatorBridgeError> {
        self.ensure_provider_inactive(provider_id)?;
        self.packages()?
            .remove_simulator_provider(provider_id)
            .map_err(simulator_package_error)
    }

    fn registrations(
        &self,
    ) -> Result<BTreeMap<String, SimulatorProviderRegistration>, SimulatorBridgeError> {
        let mut registrations = self.inner.registrations.clone();
        if let Some(packages) = self.inner.packages.as_ref() {
            for managed in packages
                .active_managed_simulator_providers()
                .map_err(simulator_package_error)?
            {
                let provider_id = managed.manifest.id.clone();
                let registration = SimulatorProviderRegistration::from_managed_package(
                    managed.manifest,
                    managed.root,
                )?;
                registrations.insert(provider_id, registration);
            }
        }
        Ok(registrations)
    }

    fn provider_ids_checked(&self) -> Result<Vec<String>, SimulatorBridgeError> {
        self.registrations()
            .map(|registrations| registrations.into_keys().collect())
    }

    fn packages(&self) -> Result<&ExtensionPackageService, SimulatorBridgeError> {
        self.inner
            .packages
            .as_ref()
            .ok_or(SimulatorBridgeError::PackageStorageUnavailable)
    }

    fn ensure_provider_inactive(&self, provider_id: &str) -> Result<(), SimulatorBridgeError> {
        let runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)?;
        if runtime.as_ref().is_some_and(|runtime| {
            runtime.provider_id == provider_id
                && runtime
                    .process_state
                    .lock()
                    .is_ok_and(|state| state.is_active())
        }) {
            Err(SimulatorBridgeError::PackageInUse)
        } else {
            Ok(())
        }
    }

    pub fn stop(&self, provider_id: &str) -> Result<SimulatorBridgeView, SimulatorBridgeError> {
        if !self.registrations()?.contains_key(provider_id) {
            return Err(SimulatorBridgeError::UnknownProvider);
        }
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
        *runtime
            .latest_snapshot_at
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? = None;
        *runtime
            .connected_at
            .lock()
            .map_err(|_| SimulatorBridgeError::StateUnavailable)? = None;
        self.status()
    }
}

fn simulator_package_error(error: ExtensionPackageManagementError) -> SimulatorBridgeError {
    match error {
        ExtensionPackageManagementError::InvalidPackage(_) => SimulatorBridgeError::InvalidPackage,
        ExtensionPackageManagementError::VersionConflict => {
            SimulatorBridgeError::PackageVersionConflict
        }
        ExtensionPackageManagementError::RollbackUnavailable => {
            SimulatorBridgeError::RollbackUnavailable
        }
        ExtensionPackageManagementError::StateUnavailable => SimulatorBridgeError::StateUnavailable,
        ExtensionPackageManagementError::RootUnavailable
        | ExtensionPackageManagementError::StorageUnavailable
        | ExtensionPackageManagementError::FileOperation => {
            SimulatorBridgeError::PackageStorageUnavailable
        }
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
    telemetry_observer: Option<Arc<dyn SimulatorTelemetryObserver>>,
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
                        if let Ok(mut connected_at) = runtime.connected_at.lock() {
                            if state == ProviderConnectionState::Connected {
                                if connected_at.is_none() {
                                    *connected_at = Some(Instant::now());
                                }
                            } else {
                                *connected_at = None;
                            }
                        }
                    }
                    if state != ProviderConnectionState::Connected
                        && let Ok(mut latest) = runtime.latest_snapshot.lock()
                    {
                        *latest = None;
                    }
                    if state != ProviderConnectionState::Connected
                        && let Ok(mut received_at) = runtime.latest_snapshot_at.lock()
                    {
                        *received_at = None;
                    }
                    if let Ok(mut last_code) = runtime.last_code.lock() {
                        *last_code = Some(code);
                    }
                }
                ValidatedProviderEvent::Telemetry(snapshot) => {
                    if let Some(observer) = telemetry_observer.as_ref() {
                        observer.observe(&runtime.provider_id, &snapshot);
                    }
                    if let Ok(mut latest) = runtime.latest_snapshot.lock() {
                        *latest = Some(snapshot);
                    }
                    if let Ok(mut received_at) = runtime.latest_snapshot_at.lock() {
                        *received_at = Some(Instant::now());
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
    if let Ok(mut received_at) = runtime.latest_snapshot_at.lock() {
        *received_at = None;
    }
    if let Ok(mut connected_at) = runtime.connected_at.lock() {
        *connected_at = None;
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

fn telemetry_is_stale(
    connection_state: ProviderConnectionState,
    snapshot_age: Option<Duration>,
    connected_age: Option<Duration>,
) -> bool {
    connection_state == ProviderConnectionState::Connected
        && snapshot_age
            .or(connected_age)
            .is_some_and(|age| age > TELEMETRY_STALE_AFTER)
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

use crate::{FleetSnapshotView, OnAirSession, SimulatorBridgeService, SnapshotAvailability};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_plugin_protocol::{
    HostMessage, MAX_MAP_LAYERS_PER_PLUGIN, MapLayerSpec, PLUGIN_API_VERSION, Permission,
    PluginCompany, PluginFleetSnapshot, PluginManifest, PluginMessage, PluginRuntime,
    PluginSnapshotAvailability, ProtocolEnvelope, read_frame, write_frame,
};
use wyrmgrid_storage::Store;

const MAX_INSTALLED_PLUGINS: usize = 128;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const BUNDLED_PLUGIN_ID: &str = "org.wyrmgrid.example.fleet-locations";
const PYTHON_BOOTSTRAP: &str = "import runpy,sys;sys.path.insert(0,sys.argv[1]);runpy.run_path(sys.argv[2],run_name='__main__')";

const BUNDLED_MANIFEST: &str =
    include_str!("../../../examples/plugins/fleet-locations/plugin.json");
const BUNDLED_ENTRY_POINT: &str =
    include_str!("../../../examples/plugins/fleet-locations/src/main.py");
const BUNDLED_PYTHON_SDK: &str = include_str!("../../../sdk/python/wyrmgrid_sdk/__init__.py");

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PluginError {
    #[error("The local plugin directory is unavailable.")]
    RootUnavailable,
    #[error("WyrmGrid could not read or save plugin permissions.")]
    StorageUnavailable,
    #[error("One or more installed plugins has invalid metadata or files.")]
    InvalidPlugin,
    #[error("That plugin is not installed.")]
    UnknownPlugin,
    #[error("That plugin uses a runtime WyrmGrid does not support yet.")]
    UnsupportedRuntime,
    #[error("This plugin requests capabilities that WyrmGrid does not support yet.")]
    UnsupportedCapability,
    #[error("Approve every requested capability before starting this plugin.")]
    PermissionRequired,
    #[error("That plugin is already running.")]
    AlreadyRunning,
    #[error("That plugin is not running.")]
    NotRunning,
    #[error("Python 3 is required to run this plugin.")]
    RuntimeUnavailable,
    #[error("WyrmGrid could not start the plugin process.")]
    LaunchFailed,
    #[error("The plugin did not complete its startup handshake.")]
    HandshakeFailed,
    #[error("The plugin stopped because it sent an invalid or unauthorized message.")]
    ProtocolViolation,
    #[error("The local plugin supervisor is unavailable.")]
    StateUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginProcessState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl PluginProcessState {
    fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub runtime: Option<PluginRuntime>,
    pub requested_permissions: Vec<Permission>,
    pub granted_permissions: Vec<Permission>,
    pub state: PluginProcessState,
    pub published_layer_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublishedPluginLayer {
    pub plugin_id: String,
    pub plugin_name: String,
    pub layer: MapLayerSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginHostView {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    pub plugins: Vec<PluginView>,
    pub layers: Vec<PublishedPluginLayer>,
}

#[derive(Clone)]
pub struct PluginService {
    inner: Arc<PluginServiceInner>,
}

struct PluginServiceInner {
    root: Option<PathBuf>,
    initialization_error: Option<String>,
    store: Store,
    onair: OnAirSession,
    simulator: SimulatorBridgeService,
    runtimes: Mutex<BTreeMap<String, Arc<RunningPlugin>>>,
}

struct RunningPlugin {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    state: Mutex<PluginProcessState>,
    last_error: Mutex<Option<String>>,
    layers: Mutex<BTreeMap<String, MapLayerSpec>>,
    outgoing_sequence: Mutex<u64>,
    last_fleet_observation: Mutex<Option<String>>,
    last_simulator_snapshot: Mutex<Option<(String, u64)>>,
    granted_permissions: BTreeSet<Permission>,
}

struct InstalledPlugin {
    manifest: PluginManifest,
    directory: PathBuf,
    entry_point: PathBuf,
}

impl PluginService {
    pub fn new(
        root: Option<PathBuf>,
        store: Store,
        onair: OnAirSession,
        simulator: SimulatorBridgeService,
    ) -> Self {
        let (root, initialization_error) = match root {
            Some(root) => match initialize_plugin_root(&root) {
                Ok(()) => (Some(root), None),
                Err(_) => (
                    None,
                    Some("The local plugin directory could not be prepared.".to_owned()),
                ),
            },
            None => (
                None,
                Some("The local plugin directory is unavailable.".to_owned()),
            ),
        };
        Self {
            inner: Arc::new(PluginServiceInner {
                root,
                initialization_error,
                store,
                onair,
                simulator,
                runtimes: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    pub fn status(&self) -> Result<PluginHostView, PluginError> {
        let Some(root) = self.inner.root.as_deref() else {
            return Ok(PluginHostView {
                available: false,
                notice: self.inner.initialization_error.clone(),
                plugins: Vec::new(),
                layers: Vec::new(),
            });
        };

        let (installed, invalid_found) = discover_plugins(root)?;
        let runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        let mut plugins = Vec::with_capacity(installed.len());
        let mut published_layers = Vec::new();
        for plugin in installed {
            let granted_permissions = self.grants_for(&plugin.manifest)?;
            let runtime = runtimes.get(&plugin.manifest.id);
            let (state, last_error, layer_count) = match runtime {
                Some(runtime) => (
                    *runtime
                        .state
                        .lock()
                        .map_err(|_| PluginError::StateUnavailable)?,
                    runtime
                        .last_error
                        .lock()
                        .map_err(|_| PluginError::StateUnavailable)?
                        .clone(),
                    runtime
                        .layers
                        .lock()
                        .map_err(|_| PluginError::StateUnavailable)?
                        .len(),
                ),
                None => (PluginProcessState::Stopped, None, 0),
            };
            if let Some(runtime) = runtime {
                for layer in runtime
                    .layers
                    .lock()
                    .map_err(|_| PluginError::StateUnavailable)?
                    .values()
                {
                    published_layers.push(PublishedPluginLayer {
                        plugin_id: plugin.manifest.id.clone(),
                        plugin_name: plugin.manifest.name.clone(),
                        layer: layer.clone(),
                    });
                }
            }
            plugins.push(PluginView {
                id: plugin.manifest.id,
                name: plugin.manifest.name,
                version: plugin.manifest.version,
                author: plugin.manifest.author,
                runtime: plugin.manifest.runtime,
                requested_permissions: plugin.manifest.permissions,
                granted_permissions: granted_permissions.iter().copied().collect(),
                state,
                published_layer_count: layer_count,
                last_error,
            });
        }

        Ok(PluginHostView {
            available: true,
            notice: invalid_found
                .then(|| "One or more invalid plugin folders were ignored.".to_owned()),
            plugins,
            layers: published_layers,
        })
    }

    pub fn approve_requested_permissions(
        &self,
        plugin_id: &str,
    ) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        ensure_supported_permissions(&plugin.manifest.permissions)?;
        let permission_names = plugin
            .manifest
            .permissions
            .iter()
            .map(|permission| permission.as_str().to_owned())
            .collect::<Vec<_>>();
        self.inner
            .store
            .replace_plugin_permission_records(plugin_id, &permission_names)
            .map_err(|_| PluginError::StorageUnavailable)?;
        self.status()
    }

    pub fn revoke_permissions(&self, plugin_id: &str) -> Result<PluginHostView, PluginError> {
        self.find_plugin(plugin_id)?;
        if self.runtime_state(plugin_id)?.is_active() {
            self.stop(plugin_id)?;
        }
        self.inner
            .store
            .replace_plugin_permission_records(plugin_id, &[])
            .map_err(|_| PluginError::StorageUnavailable)?;
        self.status()
    }

    pub fn start(&self, plugin_id: &str) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        ensure_supported_permissions(&plugin.manifest.permissions)?;
        let granted = self.grants_for(&plugin.manifest)?;
        let requested = plugin
            .manifest
            .permissions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if requested
            .iter()
            .any(|permission| !granted.contains(permission))
        {
            return Err(PluginError::PermissionRequired);
        }
        if self.runtime_state(plugin_id)?.is_active() {
            return Err(PluginError::AlreadyRunning);
        }
        if plugin.manifest.runtime != Some(PluginRuntime::Python) {
            return Err(PluginError::UnsupportedRuntime);
        }

        let mut child = spawn_python(&plugin)?;
        let stdin = child.stdin.take().ok_or(PluginError::LaunchFailed)?;
        let stdout = child.stdout.take().ok_or(PluginError::LaunchFailed)?;
        let runtime = Arc::new(RunningPlugin {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            state: Mutex::new(PluginProcessState::Starting),
            last_error: Mutex::new(None),
            layers: Mutex::new(BTreeMap::new()),
            outgoing_sequence: Mutex::new(1),
            last_fleet_observation: Mutex::new(None),
            last_simulator_snapshot: Mutex::new(None),
            granted_permissions: granted.clone(),
        });
        self.inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .insert(plugin_id.to_owned(), Arc::clone(&runtime));

        let (ready_sender, ready_receiver) = mpsc::channel();
        spawn_plugin_reader(
            plugin.manifest.id.clone(),
            requested.clone(),
            stdout,
            Arc::clone(&runtime),
            ready_sender,
        );
        self.send_message(
            &runtime,
            HostMessage::Hello {
                host_version: env!("CARGO_PKG_VERSION").to_owned(),
                plugin_id: plugin.manifest.id.clone(),
                granted_permissions: granted.iter().copied().collect(),
            },
        )?;

        if ready_receiver.recv_timeout(STARTUP_TIMEOUT) != Ok(true) {
            fail_runtime(
                &runtime,
                "The plugin did not complete its startup handshake.",
            );
            return Err(PluginError::HandshakeFailed);
        }
        *runtime
            .state
            .lock()
            .map_err(|_| PluginError::StateUnavailable)? = PluginProcessState::Running;
        self.send_fleet_if_changed(&runtime)?;
        self.send_simulator_if_changed(&runtime)?;
        self.spawn_snapshot_poller(runtime);
        self.status()
    }

    pub fn stop(&self, plugin_id: &str) -> Result<PluginHostView, PluginError> {
        self.find_plugin(plugin_id)?;
        let runtime = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .get(plugin_id)
            .cloned()
            .ok_or(PluginError::NotRunning)?;
        if !runtime_state(&runtime)?.is_active() {
            return Err(PluginError::NotRunning);
        }
        *runtime
            .state
            .lock()
            .map_err(|_| PluginError::StateUnavailable)? = PluginProcessState::Stopping;
        let _ = self.send_message(&runtime, HostMessage::Shutdown);
        stop_process(&runtime)?;
        runtime
            .layers
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .clear();
        *runtime
            .state
            .lock()
            .map_err(|_| PluginError::StateUnavailable)? = PluginProcessState::Stopped;
        self.status()
    }

    fn find_plugin(&self, plugin_id: &str) -> Result<InstalledPlugin, PluginError> {
        let root = self
            .inner
            .root
            .as_deref()
            .ok_or(PluginError::RootUnavailable)?;
        discover_plugins(root)?
            .0
            .into_iter()
            .find(|plugin| plugin.manifest.id == plugin_id)
            .ok_or(PluginError::UnknownPlugin)
    }

    fn grants_for(&self, manifest: &PluginManifest) -> Result<BTreeSet<Permission>, PluginError> {
        let requested = manifest
            .permissions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        self.inner
            .store
            .list_plugin_permission_records(&manifest.id)
            .map_err(|_| PluginError::StorageUnavailable)
            .map(|records| {
                records
                    .iter()
                    .filter_map(|record| Permission::from_name(record))
                    .filter(|permission| requested.contains(permission))
                    .collect()
            })
    }

    fn runtime_state(&self, plugin_id: &str) -> Result<PluginProcessState, PluginError> {
        self.inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .get(plugin_id)
            .map(runtime_state)
            .transpose()
            .map(|state| state.unwrap_or(PluginProcessState::Stopped))
    }

    fn send_message(
        &self,
        runtime: &Arc<RunningPlugin>,
        message: HostMessage,
    ) -> Result<(), PluginError> {
        let mut sequence = runtime
            .outgoing_sequence
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        let mut stdin = runtime
            .stdin
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        write_frame(&mut *stdin, &ProtocolEnvelope::new(*sequence, message)).map_err(|_| {
            fail_runtime(runtime, "The plugin process stopped unexpectedly.");
            PluginError::ProtocolViolation
        })?;
        *sequence += 1;
        Ok(())
    }

    fn send_fleet_if_changed(&self, runtime: &Arc<RunningPlugin>) -> Result<(), PluginError> {
        if !runtime
            .granted_permissions
            .contains(&Permission::OnAirFleetRead)
        {
            return Ok(());
        }
        let Some(snapshot) = self
            .inner
            .onair
            .fleet_snapshot()
            .map_err(|_| PluginError::StateUnavailable)?
        else {
            return Ok(());
        };
        let observed_at = snapshot.snapshot.provenance.observed_at.to_rfc3339();
        let mut last_observation = runtime
            .last_fleet_observation
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        if last_observation.as_deref() == Some(observed_at.as_str()) {
            return Ok(());
        }
        self.send_message(
            runtime,
            HostMessage::FleetSnapshot {
                snapshot: plugin_fleet_snapshot(snapshot),
            },
        )?;
        *last_observation = Some(observed_at);
        Ok(())
    }

    fn send_simulator_if_changed(&self, runtime: &Arc<RunningPlugin>) -> Result<(), PluginError> {
        if !runtime
            .granted_permissions
            .contains(&Permission::SimulatorTelemetryRead)
        {
            return Ok(());
        }
        let Some(snapshot) = self
            .inner
            .simulator
            .latest_snapshot()
            .map_err(|_| PluginError::StateUnavailable)?
        else {
            return Ok(());
        };
        let snapshot_key = (snapshot.provenance.provider.clone(), snapshot.sequence);
        let mut last_snapshot = runtime
            .last_simulator_snapshot
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        if last_snapshot.as_ref() == Some(&snapshot_key) {
            return Ok(());
        }
        self.send_message(
            runtime,
            HostMessage::SimulatorTelemetrySnapshot {
                snapshot: Box::new(snapshot),
            },
        )?;
        *last_snapshot = Some(snapshot_key);
        Ok(())
    }

    fn spawn_snapshot_poller(&self, runtime: Arc<RunningPlugin>) {
        let service = Arc::downgrade(&self.inner);
        thread::spawn(move || {
            while runtime_state(&runtime) == Ok(PluginProcessState::Running) {
                thread::sleep(SNAPSHOT_POLL_INTERVAL);
                if runtime_state(&runtime) != Ok(PluginProcessState::Running) {
                    return;
                }
                let Some(inner) = service.upgrade() else {
                    return;
                };
                let service = PluginService { inner };
                if service.send_fleet_if_changed(&runtime).is_err()
                    || service.send_simulator_if_changed(&runtime).is_err()
                {
                    return;
                }
            }
        });
    }
}

impl Drop for PluginServiceInner {
    fn drop(&mut self) {
        if let Ok(runtimes) = self.runtimes.lock() {
            for runtime in runtimes.values() {
                let _ = stop_process(runtime);
            }
        }
    }
}

fn initialize_plugin_root(root: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(root)?;
    let plugin_root = root.join(BUNDLED_PLUGIN_ID);
    let source_root = plugin_root.join("src");
    let sdk_root = source_root.join("wyrmgrid_sdk");
    fs::create_dir_all(&sdk_root)?;
    write_bundled_file(&plugin_root.join("plugin.json"), BUNDLED_MANIFEST)?;
    write_bundled_file(&source_root.join("main.py"), BUNDLED_ENTRY_POINT)?;
    write_bundled_file(&sdk_root.join("__init__.py"), BUNDLED_PYTHON_SDK)
}

fn write_bundled_file(path: &Path, contents: &str) -> Result<(), io::Error> {
    if !path.exists() {
        fs::write(path, contents)?;
    }
    Ok(())
}

fn discover_plugins(root: &Path) -> Result<(Vec<InstalledPlugin>, bool), PluginError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| PluginError::RootUnavailable)?;
    let entries = fs::read_dir(&canonical_root).map_err(|_| PluginError::RootUnavailable)?;
    let mut installed = Vec::new();
    let mut invalid_found = false;
    for entry in entries.take(MAX_INSTALLED_PLUGINS + 1) {
        if installed.len() >= MAX_INSTALLED_PLUGINS {
            invalid_found = true;
            break;
        }
        let Ok(entry) = entry else {
            invalid_found = true;
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            invalid_found = true;
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        match read_installed_plugin(&canonical_root, &entry.path()) {
            Ok(plugin) => installed.push(plugin),
            Err(_) => invalid_found = true,
        }
    }
    installed.sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    Ok((installed, invalid_found))
}

fn read_installed_plugin(root: &Path, directory: &Path) -> Result<InstalledPlugin, PluginError> {
    let canonical_directory = directory
        .canonicalize()
        .map_err(|_| PluginError::InvalidPlugin)?;
    if !canonical_directory.starts_with(root) {
        return Err(PluginError::InvalidPlugin);
    }
    let manifest_path = canonical_directory.join("plugin.json");
    let metadata = fs::symlink_metadata(&manifest_path).map_err(|_| PluginError::InvalidPlugin)?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(PluginError::InvalidPlugin);
    }
    let manifest: PluginManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).map_err(|_| PluginError::InvalidPlugin)?,
    )
    .map_err(|_| PluginError::InvalidPlugin)?;
    manifest
        .validate()
        .map_err(|_| PluginError::InvalidPlugin)?;
    if directory.file_name().and_then(|name| name.to_str()) != Some(manifest.id.as_str()) {
        return Err(PluginError::InvalidPlugin);
    }
    let entry_point = canonical_directory
        .join(&manifest.entry_point)
        .canonicalize()
        .map_err(|_| PluginError::InvalidPlugin)?;
    if !entry_point.starts_with(&canonical_directory) || !entry_point.is_file() {
        return Err(PluginError::InvalidPlugin);
    }
    Ok(InstalledPlugin {
        manifest,
        directory: canonical_directory,
        entry_point,
    })
}

fn ensure_supported_permissions(permissions: &[Permission]) -> Result<(), PluginError> {
    permissions
        .iter()
        .all(|permission| {
            matches!(
                permission,
                Permission::OnAirFleetRead
                    | Permission::MapLayersPublish
                    | Permission::SimulatorTelemetryRead
            )
        })
        .then_some(())
        .ok_or(PluginError::UnsupportedCapability)
}

fn spawn_python(plugin: &InstalledPlugin) -> Result<Child, PluginError> {
    #[cfg(windows)]
    let candidates: &[(&str, &[&str])] = &[("py", &["-3"]), ("python", &[])];
    #[cfg(not(windows))]
    let candidates: &[(&str, &[&str])] = &[("python3", &[]), ("python", &[])];

    let mut runtime_found = false;
    for (program, prefix_arguments) in candidates {
        let mut command = Command::new(program);
        command
            .args(*prefix_arguments)
            .arg("-I")
            .arg("-c")
            .arg(PYTHON_BOOTSTRAP)
            .arg(
                plugin
                    .entry_point
                    .parent()
                    .ok_or(PluginError::InvalidPlugin)?,
            )
            .arg(&plugin.entry_point)
            .current_dir(&plugin.directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_clear()
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PYTHONUTF8", "1");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
            for key in ["SystemRoot", "LOCALAPPDATA", "APPDATA", "USERPROFILE"] {
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            }
        }
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => {
                runtime_found = true;
                break;
            }
        }
    }
    if runtime_found {
        Err(PluginError::LaunchFailed)
    } else {
        Err(PluginError::RuntimeUnavailable)
    }
}

fn spawn_plugin_reader(
    expected_plugin_id: String,
    granted_permissions: BTreeSet<Permission>,
    mut stdout: std::process::ChildStdout,
    runtime: Arc<RunningPlugin>,
    ready_sender: Sender<bool>,
) {
    thread::spawn(move || {
        let mut ready_sender = Some(ready_sender);
        let mut ready = false;
        let mut last_sequence = 0_u64;
        loop {
            let envelope: ProtocolEnvelope<PluginMessage> = match read_frame(&mut stdout) {
                Ok(envelope) => envelope,
                Err(_) => {
                    let stopping = runtime_state(&runtime) == Ok(PluginProcessState::Stopping);
                    if !stopping {
                        fail_runtime(&runtime, "The plugin process stopped unexpectedly.");
                    }
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            };
            if envelope.validate_header().is_err() || envelope.sequence <= last_sequence {
                fail_runtime(&runtime, "The plugin sent an invalid message envelope.");
                if let Some(sender) = ready_sender.take() {
                    let _ = sender.send(false);
                }
                return;
            }
            last_sequence = envelope.sequence;
            match envelope.payload {
                PluginMessage::Ready {
                    plugin_id,
                    api_version,
                } if !ready
                    && plugin_id == expected_plugin_id
                    && api_version == PLUGIN_API_VERSION =>
                {
                    ready = true;
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(true);
                    }
                }
                PluginMessage::PublishMapLayer { layer }
                    if ready && granted_permissions.contains(&Permission::MapLayersPublish) =>
                {
                    if layer.validate().is_err() {
                        fail_runtime(&runtime, "The plugin published an invalid map layer.");
                        return;
                    }
                    let mut layers = match runtime.layers.lock() {
                        Ok(layers) => layers,
                        Err(_) => {
                            fail_runtime(&runtime, "The plugin supervisor became unavailable.");
                            return;
                        }
                    };
                    if !layers.contains_key(&layer.id) && layers.len() >= MAX_MAP_LAYERS_PER_PLUGIN
                    {
                        drop(layers);
                        fail_runtime(&runtime, "The plugin published too many map layers.");
                        return;
                    }
                    layers.insert(layer.id.clone(), layer);
                }
                _ => {
                    fail_runtime(
                        &runtime,
                        "The plugin sent an invalid or unauthorized message.",
                    );
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            }
        }
    });
}

fn runtime_state(runtime: &Arc<RunningPlugin>) -> Result<PluginProcessState, PluginError> {
    runtime
        .state
        .lock()
        .map(|state| *state)
        .map_err(|_| PluginError::StateUnavailable)
}

fn fail_runtime(runtime: &Arc<RunningPlugin>, message: &str) {
    if let Ok(mut state) = runtime.state.lock() {
        *state = PluginProcessState::Failed;
    }
    if let Ok(mut last_error) = runtime.last_error.lock() {
        *last_error = Some(message.to_owned());
    }
    if let Ok(mut layers) = runtime.layers.lock() {
        layers.clear();
    }
    if let Ok(mut child) = runtime.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn stop_process(runtime: &Arc<RunningPlugin>) -> Result<(), PluginError> {
    let mut child = runtime
        .child
        .lock()
        .map_err(|_| PluginError::StateUnavailable)?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if started.elapsed() < SHUTDOWN_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
        }
    }
}

fn plugin_fleet_snapshot(view: FleetSnapshotView) -> PluginFleetSnapshot {
    PluginFleetSnapshot {
        company: PluginCompany {
            name: view.company.name,
            airline_code: view.company.airline_code,
        },
        aircraft: view.snapshot.value,
        provenance: view.snapshot.provenance,
        availability: match view.availability {
            SnapshotAvailability::Live => PluginSnapshotAvailability::Live,
            SnapshotAvailability::Cached => PluginSnapshotAvailability::Cached,
            SnapshotAvailability::Offline => PluginSnapshotAvailability::Offline,
        },
    }
}

#[cfg(test)]
#[path = "tests/plugins.rs"]
mod tests;

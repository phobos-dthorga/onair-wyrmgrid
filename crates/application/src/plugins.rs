use crate::{FleetSnapshotView, OnAirSession, SimulatorBridgeService, SnapshotAvailability};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_domain::{GlobalWeatherLayerSnapshot, GlobalWeatherTimeKind, WeatherSnapshot};
use wyrmgrid_plugin_protocol::{
    HostMessage, MAX_MAP_LAYERS_PER_PLUGIN, MapLayerSpec, PLUGIN_API_VERSION, Permission,
    PluginCompany, PluginFleetSnapshot, PluginManifest, PluginMessage, PluginRuntime,
    PluginSnapshotAvailability, PluginWeatherProduct, PluginWeatherResponse, ProtocolEnvelope,
    WeatherCapability, WeatherGridRequestPoint, WeatherQuery, WeatherRequest, WeatherTileAddress,
    WeatherTimeWindow, WeatherUnavailableCode, read_frame, write_frame,
};
use wyrmgrid_storage::{PluginConfigurationRecord, PluginPreferencesRecord, Store};

use crate::authorization::{
    AuthorizationError, AuthorizationGrantLifetime, AuthorizationRuntime, AuthorizationService,
    AuthorizationSubject,
};

const MAX_INSTALLED_PLUGINS: usize = 128;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MODEL_WEATHER_REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RADAR_WEATHER_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
const FORECAST_REFRESH_SETTING_KEY: &str = "forecast_refresh_minutes";
const RADAR_REFRESH_SETTING_KEY: &str = "radar_refresh_minutes";
const FORECAST_REFRESH_OPTIONS: [u16; 4] = [15, 30, 60, 120];
const RADAR_REFRESH_OPTIONS: [u16; 4] = [5, 10, 15, 30];
const AIRPORT_WEATHER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
const GLOBAL_WEATHER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_WEATHER_LAYERS_PER_PLUGIN: usize = 4;
const MAX_RADAR_FRAMES_PER_LAYER: usize = 6;
const BUNDLED_PLUGIN_ID: &str = "org.wyrmgrid.example.fleet-locations";
const OPEN_METEO_PLUGIN_ID: &str = "org.wyrmgrid.provider.open-meteo";
const AVIATION_WEATHER_PLUGIN_ID: &str = "org.wyrmgrid.provider.aviation-weather";
const RAINVIEWER_PLUGIN_ID: &str = "org.wyrmgrid.provider.rainviewer";
const PYTHON_BOOTSTRAP: &str = "import runpy,sys;sys.path.insert(0,sys.argv[1]);runpy.run_path(sys.argv[2],run_name='__main__')";

const BUNDLED_MANIFEST: &str =
    include_str!("../../../examples/plugins/fleet-locations/plugin.json");
const BUNDLED_ENTRY_POINT: &str =
    include_str!("../../../examples/plugins/fleet-locations/src/main.py");
const BUNDLED_PYTHON_SDK: &str = include_str!("../../../sdk/python/wyrmgrid_sdk/__init__.py");
const OPEN_METEO_MANIFEST: &str = include_str!("../../../plugins/open-meteo/plugin.json");
const OPEN_METEO_ENTRY_POINT: &str = include_str!("../../../plugins/open-meteo/src/main.py");
const AVIATION_WEATHER_MANIFEST: &str =
    include_str!("../../../plugins/aviation-weather/plugin.json");
const AVIATION_WEATHER_ENTRY_POINT: &str =
    include_str!("../../../plugins/aviation-weather/src/main.py");
const RAINVIEWER_MANIFEST: &str = include_str!("../../../plugins/rainviewer/plugin.json");
const RAINVIEWER_ENTRY_POINT: &str = include_str!("../../../plugins/rainviewer/src/main.py");

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
    #[error("Choose standing access before enabling automatic plugin startup.")]
    StandingPermissionRequired,
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
    #[error("That plugin setting is not available.")]
    UnknownConfiguration,
    #[error("That plugin setting value is not supported.")]
    InvalidConfiguration,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PluginWeatherError {
    #[error("No running plugin provides that weather product.")]
    ProviderUnavailable,
    #[error("The weather provider is offline.")]
    Offline,
    #[error("The weather provider request timed out.")]
    TimedOut,
    #[error("The weather provider is rate-limiting requests.")]
    RateLimited,
    #[error("The weather provider returned an invalid response.")]
    InvalidResponse,
    #[error("The weather provider has no data for this request.")]
    NoData,
    #[error("The plugin weather service is unavailable.")]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDiagnosticEvent {
    pub plugin_id: String,
    pub level: &'static str,
    pub code: &'static str,
    pub operation: &'static str,
    pub message: &'static str,
    pub reportable: bool,
}

pub trait PluginDiagnosticObserver: Send + Sync + 'static {
    fn observe(&self, event: &PluginDiagnosticEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginRuntimeFailure {
    HandshakeFailed,
    ProcessStopped,
    ProtocolEnvelopeInvalid,
    MapLayerInvalid,
    SupervisorUnavailable,
    MapLayerLimitExceeded,
    WeatherRequestTimedOut,
    WeatherCapabilityUndeclared,
    WeatherRequestUnknown,
    WeatherProductInvalid,
    WeatherProductCapabilityMismatch,
    WeatherProductRequestMismatch,
    WeatherLayerLimitExceeded,
    MessageUnauthorized,
}

impl PluginRuntimeFailure {
    fn diagnostic(self, plugin_id: &str) -> PluginDiagnosticEvent {
        let (code, operation, message, reportable) = match self {
            Self::HandshakeFailed => (
                "plugin.handshake_failed",
                "plugin_startup",
                "The plugin did not complete its startup handshake.",
                false,
            ),
            Self::ProcessStopped => (
                "plugin.process_stopped_unexpectedly",
                "plugin_runtime",
                "The plugin process stopped unexpectedly.",
                true,
            ),
            Self::ProtocolEnvelopeInvalid => (
                "plugin.protocol_envelope_invalid",
                "plugin_protocol",
                "The plugin sent an invalid message envelope.",
                true,
            ),
            Self::MapLayerInvalid => (
                "plugin.map_layer_invalid",
                "plugin_publication",
                "The plugin published an invalid map layer.",
                true,
            ),
            Self::SupervisorUnavailable => (
                "plugin.supervisor_unavailable",
                "plugin_supervision",
                "The plugin supervisor became unavailable.",
                true,
            ),
            Self::MapLayerLimitExceeded => (
                "plugin.map_layer_limit_exceeded",
                "plugin_publication",
                "The plugin published too many map layers.",
                true,
            ),
            Self::WeatherRequestTimedOut => (
                "plugin.weather_request_timed_out",
                "plugin_weather",
                "The plugin weather request exceeded its deadline.",
                false,
            ),
            Self::WeatherCapabilityUndeclared => (
                "plugin.weather_capability_undeclared",
                "plugin_weather",
                "The plugin published a weather capability it did not declare.",
                true,
            ),
            Self::WeatherRequestUnknown => (
                "plugin.weather_request_unknown",
                "plugin_weather",
                "The plugin answered an unknown weather request.",
                true,
            ),
            Self::WeatherProductInvalid => (
                "plugin.weather_product_invalid",
                "plugin_weather",
                "The plugin published a structurally invalid weather product.",
                true,
            ),
            Self::WeatherProductCapabilityMismatch => (
                "plugin.weather_product_capability_mismatch",
                "plugin_weather",
                "The plugin published the wrong kind of weather product.",
                true,
            ),
            Self::WeatherProductRequestMismatch => (
                "plugin.weather_product_request_mismatch",
                "plugin_weather",
                "The plugin weather product did not match the bounded host request.",
                true,
            ),
            Self::WeatherLayerLimitExceeded => (
                "plugin.weather_layer_limit_exceeded",
                "plugin_weather",
                "The plugin published too many weather layers.",
                true,
            ),
            Self::MessageUnauthorized => (
                "plugin.message_unauthorized",
                "plugin_protocol",
                "The plugin sent an invalid or unauthorized message.",
                true,
            ),
        };
        PluginDiagnosticEvent {
            plugin_id: plugin_id.to_owned(),
            level: "error",
            code,
            operation,
            message,
            reportable,
        }
    }
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
    pub weather_capabilities: Vec<WeatherCapability>,
    pub network_origins: Vec<String>,
    pub requested_permissions: Vec<Permission>,
    pub granted_permissions: Vec<Permission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_lifetime: Option<AuthorizationGrantLifetime>,
    pub start_with_wyrmgrid: bool,
    pub configuration: Vec<PluginSettingView>,
    pub state: PluginProcessState,
    pub published_layer_count: usize,
    pub published_weather_layer_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginSettingChoice {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginSettingView {
    pub key: String,
    pub value: String,
    pub choices: Vec<PluginSettingChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginAutoStartFailure {
    pub plugin_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginAutoStartOutcome {
    pub started_plugin_ids: Vec<String>,
    pub failures: Vec<PluginAutoStartFailure>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublishedPluginLayer {
    pub plugin_id: String,
    pub plugin_name: String,
    pub layer: MapLayerSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublishedPluginWeatherLayer {
    pub plugin_id: String,
    pub plugin_name: String,
    pub layer: GlobalWeatherLayerSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginHostView {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    pub plugins: Vec<PluginView>,
    pub layers: Vec<PublishedPluginLayer>,
    pub weather_layers: Vec<PublishedPluginWeatherLayer>,
}

#[derive(Clone)]
pub struct PluginService {
    inner: Arc<PluginServiceInner>,
}

struct PluginServiceInner {
    root: Option<PathBuf>,
    initialization_error: Option<String>,
    authorization: AuthorizationService<Store>,
    preferences: Store,
    onair: OnAirSession,
    simulator: SimulatorBridgeService,
    diagnostic_observer: Option<Arc<dyn PluginDiagnosticObserver>>,
    runtimes: Mutex<BTreeMap<String, Arc<RunningPlugin>>>,
    startup_failures: Mutex<BTreeMap<String, String>>,
}

struct RunningPlugin {
    plugin_id: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    state: Mutex<PluginProcessState>,
    failure_recorded: AtomicBool,
    last_error: Mutex<Option<String>>,
    layers: Mutex<BTreeMap<String, MapLayerSpec>>,
    weather_layers: Mutex<BTreeMap<String, Vec<GlobalWeatherLayerSnapshot>>>,
    pending_weather: Mutex<BTreeMap<String, PendingWeatherRequest>>,
    last_weather_requests: Mutex<BTreeMap<WeatherCapability, Instant>>,
    weather_refresh_intervals: Mutex<BTreeMap<WeatherCapability, Duration>>,
    weather_request_sequence: Mutex<u64>,
    outgoing_sequence: Mutex<u64>,
    last_fleet_observation: Mutex<Option<String>>,
    last_simulator_snapshot: Mutex<Option<(String, u64)>>,
    granted_permissions: BTreeSet<Permission>,
    weather_capabilities: BTreeSet<WeatherCapability>,
    grant_lifetime: AuthorizationGrantLifetime,
    diagnostic_observer: Option<Arc<dyn PluginDiagnosticObserver>>,
}

struct PendingWeatherRequest {
    capability: WeatherCapability,
    query: WeatherQuery,
    queued_at: Instant,
    response_sender: Option<Sender<PluginWeatherResponse>>,
}

struct InstalledPlugin {
    manifest: PluginManifest,
    directory: PathBuf,
    entry_point: PathBuf,
}

#[derive(Clone, Copy)]
struct PluginSettingDefinition {
    key: &'static str,
    capability: WeatherCapability,
    default_minutes: u16,
    options: &'static [u16],
}

impl PluginService {
    pub fn new(
        root: Option<PathBuf>,
        store: Store,
        onair: OnAirSession,
        simulator: SimulatorBridgeService,
    ) -> Self {
        Self::with_authorization_runtime(
            root,
            store,
            onair,
            simulator,
            AuthorizationRuntime::default(),
        )
    }

    pub fn with_authorization_runtime(
        root: Option<PathBuf>,
        store: Store,
        onair: OnAirSession,
        simulator: SimulatorBridgeService,
        authorization_runtime: AuthorizationRuntime,
    ) -> Self {
        Self::with_authorization_runtime_and_diagnostics(
            root,
            store,
            onair,
            simulator,
            authorization_runtime,
            None,
        )
    }

    pub fn with_authorization_runtime_and_diagnostics(
        root: Option<PathBuf>,
        store: Store,
        onair: OnAirSession,
        simulator: SimulatorBridgeService,
        authorization_runtime: AuthorizationRuntime,
        diagnostic_observer: Option<Arc<dyn PluginDiagnosticObserver>>,
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
                authorization: AuthorizationService::with_runtime(
                    store.clone(),
                    authorization_runtime,
                ),
                preferences: store,
                onair,
                simulator,
                diagnostic_observer,
                runtimes: Mutex::new(BTreeMap::new()),
                startup_failures: Mutex::new(BTreeMap::new()),
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
                weather_layers: Vec::new(),
            });
        };

        let (installed, invalid_found) = discover_plugins(root)?;
        let runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        let startup_failures = self
            .inner
            .startup_failures
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        let mut plugins = Vec::with_capacity(installed.len());
        let mut published_layers = Vec::new();
        let mut published_weather_layers = Vec::new();
        for plugin in installed {
            let stored_granted_permissions = self.grants_for(&plugin.manifest)?;
            let stored_grant_lifetime = self.grant_lifetime_for(&plugin.manifest)?;
            let runtime = runtimes.get(&plugin.manifest.id);
            let (state, last_error, layer_count, weather_layer_count) = match runtime {
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
                    runtime
                        .weather_layers
                        .lock()
                        .map_err(|_| PluginError::StateUnavailable)?
                        .values()
                        .map(Vec::len)
                        .sum(),
                ),
                None => (
                    PluginProcessState::Stopped,
                    startup_failures.get(&plugin.manifest.id).cloned(),
                    0,
                    0,
                ),
            };
            let granted_permissions = runtime
                .filter(|_| state.is_active())
                .map(|runtime| runtime.granted_permissions.clone())
                .unwrap_or(stored_granted_permissions);
            let grant_lifetime = runtime
                .filter(|_| state.is_active())
                .map(|runtime| runtime.grant_lifetime)
                .or(stored_grant_lifetime);
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
                for layer in runtime
                    .weather_layers
                    .lock()
                    .map_err(|_| PluginError::StateUnavailable)?
                    .values()
                    .flatten()
                {
                    published_weather_layers.push(PublishedPluginWeatherLayer {
                        plugin_id: plugin.manifest.id.clone(),
                        plugin_name: plugin.manifest.name.clone(),
                        layer: layer.clone(),
                    });
                }
            }
            let start_with_wyrmgrid = self.start_with_wyrmgrid(&plugin.manifest)?;
            let configuration = self.configuration_for(&plugin.manifest)?;
            plugins.push(PluginView {
                id: plugin.manifest.id,
                name: plugin.manifest.name,
                version: plugin.manifest.version,
                author: plugin.manifest.author,
                runtime: plugin.manifest.runtime,
                weather_capabilities: plugin.manifest.weather_capabilities,
                network_origins: plugin.manifest.network_origins,
                requested_permissions: plugin.manifest.permissions,
                granted_permissions: granted_permissions.iter().copied().collect(),
                grant_lifetime,
                start_with_wyrmgrid,
                configuration,
                state,
                published_layer_count: layer_count,
                published_weather_layer_count: weather_layer_count,
                last_error,
            });
        }

        Ok(PluginHostView {
            available: true,
            notice: invalid_found
                .then(|| "One or more invalid plugin folders were ignored.".to_owned()),
            plugins,
            layers: published_layers,
            weather_layers: published_weather_layers,
        })
    }

    pub fn enrich_dispatch_route_weather(
        &self,
        status: &mut crate::DispatchStatus,
    ) -> Result<(), PluginError> {
        let Some(plan) = status.atlas_plan.as_ref() else {
            status.route_weather = None;
            return Ok(());
        };
        let runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?;
        let mut weather_layers = Vec::new();
        let historical = status.weather.time_basis == crate::RouteWeatherTemporalMode::Historical;
        for runtime in runtimes.values() {
            let histories = runtime
                .weather_layers
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?;
            for history in histories.values() {
                if let Some(layer) = history.last() {
                    let is_historical_layer = layer.time_scope.as_ref().is_some_and(|scope| {
                        matches!(
                            scope.kind,
                            GlobalWeatherTimeKind::HistoricalModel
                                | GlobalWeatherTimeKind::ArchivedForecast
                        )
                    });
                    if historical != is_historical_layer {
                        continue;
                    }
                    match &layer.data {
                        wyrmgrid_domain::GlobalWeatherLayerData::Grid { .. } => {
                            weather_layers.push(layer.clone());
                        }
                        wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles { .. } => {
                            weather_layers.extend(history.iter().cloned());
                        }
                    }
                }
            }
        }
        let schedule = status
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.schedule.as_ref())
            .map(|schedule| &schedule.value);
        status.route_weather = Some(crate::build_route_weather_analysis(
            plan,
            schedule,
            &weather_layers,
        ));
        Ok(())
    }

    pub fn approve_requested_permissions(
        &self,
        plugin_id: &str,
    ) -> Result<PluginHostView, PluginError> {
        self.approve_requested_permissions_with_lifetime(
            plugin_id,
            AuthorizationGrantLifetime::Standing,
        )
    }

    pub fn approve_requested_permissions_with_lifetime(
        &self,
        plugin_id: &str,
        lifetime: AuthorizationGrantLifetime,
    ) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        ensure_supported_permissions(&plugin.manifest.permissions)?;
        let subject = AuthorizationSubject::plugin(plugin_id);
        let revision = plugin_scope_revision(&plugin.manifest);
        let permission_names = requested_capability_names(&plugin.manifest);
        self.inner
            .authorization
            .approve_with_lifetime(&subject, &revision, &permission_names, lifetime)
            .map_err(plugin_authorization_error)?;
        if lifetime != AuthorizationGrantLifetime::Standing {
            self.inner
                .preferences
                .delete_plugin_preferences_record(plugin_id)
                .map_err(|_| PluginError::StorageUnavailable)?;
            self.clear_startup_failure(plugin_id)?;
        }
        self.status()
    }

    pub fn revoke_permissions(&self, plugin_id: &str) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        if self.runtime_state(plugin_id)?.is_active() {
            self.stop(plugin_id)?;
        }
        let subject = AuthorizationSubject::plugin(plugin_id);
        let revision = plugin_scope_revision(&plugin.manifest);
        self.inner
            .authorization
            .revoke(&subject, &revision)
            .map_err(plugin_authorization_error)?;
        self.inner
            .preferences
            .delete_plugin_preferences_record(plugin_id)
            .map_err(|_| PluginError::StorageUnavailable)?;
        self.clear_startup_failure(plugin_id)?;
        self.status()
    }

    pub fn set_start_with_wyrmgrid(
        &self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        if enabled {
            ensure_supported_permissions(&plugin.manifest.permissions)?;
            if plugin.manifest.runtime != Some(PluginRuntime::Python) {
                return Err(PluginError::UnsupportedRuntime);
            }
            if self.grant_lifetime_for(&plugin.manifest)?
                != Some(AuthorizationGrantLifetime::Standing)
            {
                return Err(PluginError::StandingPermissionRequired);
            }
            self.inner
                .preferences
                .save_plugin_preferences_record(&PluginPreferencesRecord {
                    plugin_id: plugin.manifest.id.clone(),
                    scope_revision: plugin_scope_revision(&plugin.manifest),
                    start_with_wyrmgrid: true,
                })
                .map_err(|_| PluginError::StorageUnavailable)?;
        } else {
            self.inner
                .preferences
                .delete_plugin_preferences_record(plugin_id)
                .map_err(|_| PluginError::StorageUnavailable)?;
            self.clear_startup_failure(plugin_id)?;
        }
        self.status()
    }

    pub fn set_configuration(
        &self,
        plugin_id: &str,
        setting_key: &str,
        value: &str,
    ) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        let definition = plugin_setting_definition(&plugin.manifest, setting_key)
            .ok_or(PluginError::UnknownConfiguration)?;
        let minutes = value
            .parse::<u16>()
            .ok()
            .filter(|value| definition.options.contains(value))
            .ok_or(PluginError::InvalidConfiguration)?;
        self.inner
            .preferences
            .save_plugin_configuration_record(&PluginConfigurationRecord {
                plugin_id: plugin.manifest.id.clone(),
                setting_key: setting_key.to_owned(),
                value: minutes.to_string(),
            })
            .map_err(|_| PluginError::StorageUnavailable)?;

        if let Some(runtime) = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .get(plugin_id)
            .cloned()
        {
            runtime
                .weather_refresh_intervals
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?
                .insert(
                    definition.capability,
                    Duration::from_secs(u64::from(minutes) * 60),
                );
        }
        self.status()
    }

    pub fn start_enabled(&self) -> Result<PluginAutoStartOutcome, PluginError> {
        let root = self
            .inner
            .root
            .as_deref()
            .ok_or(PluginError::RootUnavailable)?;
        let (installed, _) = discover_plugins(root)?;
        let mut enabled = Vec::new();
        for plugin in installed {
            if self.start_with_wyrmgrid(&plugin.manifest)? {
                enabled.push(plugin.manifest.id);
            }
        }
        let mut outcome = PluginAutoStartOutcome {
            started_plugin_ids: Vec::new(),
            failures: Vec::new(),
        };
        for plugin_id in enabled {
            match self.start(&plugin_id) {
                Ok(_) => outcome.started_plugin_ids.push(plugin_id),
                Err(error) => {
                    let message = error.to_string();
                    self.inner
                        .startup_failures
                        .lock()
                        .map_err(|_| PluginError::StateUnavailable)?
                        .insert(plugin_id.clone(), message.clone());
                    outcome
                        .failures
                        .push(PluginAutoStartFailure { plugin_id, message });
                }
            }
        }
        Ok(outcome)
    }

    pub fn start(&self, plugin_id: &str) -> Result<PluginHostView, PluginError> {
        let plugin = self.find_plugin(plugin_id)?;
        ensure_supported_permissions(&plugin.manifest.permissions)?;
        let grant_lifetime = self
            .grant_lifetime_for(&plugin.manifest)?
            .ok_or(PluginError::PermissionRequired)?;
        let requested = plugin
            .manifest
            .permissions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if self.runtime_state(plugin_id)?.is_active() {
            return Err(PluginError::AlreadyRunning);
        }
        if plugin.manifest.runtime != Some(PluginRuntime::Python) {
            return Err(PluginError::UnsupportedRuntime);
        }
        let granted = self.required_grants_for(&plugin.manifest)?;

        let mut child = spawn_python(&plugin)?;
        let stdin = child.stdin.take().ok_or(PluginError::LaunchFailed)?;
        let stdout = child.stdout.take().ok_or(PluginError::LaunchFailed)?;
        let weather_refresh_intervals = self.weather_refresh_intervals(&plugin.manifest)?;
        let runtime = Arc::new(RunningPlugin {
            plugin_id: plugin.manifest.id.clone(),
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            state: Mutex::new(PluginProcessState::Starting),
            failure_recorded: AtomicBool::new(false),
            last_error: Mutex::new(None),
            layers: Mutex::new(BTreeMap::new()),
            weather_layers: Mutex::new(BTreeMap::new()),
            pending_weather: Mutex::new(BTreeMap::new()),
            last_weather_requests: Mutex::new(BTreeMap::new()),
            weather_refresh_intervals: Mutex::new(weather_refresh_intervals),
            weather_request_sequence: Mutex::new(1),
            outgoing_sequence: Mutex::new(1),
            last_fleet_observation: Mutex::new(None),
            last_simulator_snapshot: Mutex::new(None),
            granted_permissions: granted.clone(),
            weather_capabilities: plugin
                .manifest
                .weather_capabilities
                .iter()
                .copied()
                .collect(),
            grant_lifetime,
            diagnostic_observer: self.inner.diagnostic_observer.clone(),
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
            plugin
                .manifest
                .weather_capabilities
                .iter()
                .copied()
                .collect(),
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
                weather_capabilities: if granted.contains(&Permission::WeatherDataPublish) {
                    plugin.manifest.weather_capabilities.clone()
                } else {
                    Vec::new()
                },
                network_origins: if granted.contains(&Permission::ExternalNetwork) {
                    plugin.manifest.network_origins.clone()
                } else {
                    Vec::new()
                },
            },
        )?;

        if ready_receiver.recv_timeout(STARTUP_TIMEOUT) != Ok(true) {
            fail_runtime(&runtime, PluginRuntimeFailure::HandshakeFailed);
            return Err(PluginError::HandshakeFailed);
        }
        *runtime
            .state
            .lock()
            .map_err(|_| PluginError::StateUnavailable)? = PluginProcessState::Running;
        self.clear_startup_failure(plugin_id)?;
        self.send_fleet_if_changed(&runtime)?;
        self.send_simulator_if_changed(&runtime)?;
        self.send_global_weather_if_due(&runtime)?;
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
        runtime
            .weather_layers
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .clear();
        notify_pending_weather(&runtime, WeatherUnavailableCode::ProviderUnavailable);
        *runtime
            .state
            .lock()
            .map_err(|_| PluginError::StateUnavailable)? = PluginProcessState::Stopped;
        self.status()
    }

    pub fn weather_capability_available(
        &self,
        capability: WeatherCapability,
    ) -> Result<bool, PluginWeatherError> {
        let runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginWeatherError::StateUnavailable)?;
        for runtime in runtimes.values() {
            if runtime.weather_capabilities.contains(&capability)
                && runtime_state(runtime) == Ok(PluginProcessState::Running)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn request_airport_weather(
        &self,
        stations: &[String],
        window: Option<WeatherTimeWindow>,
    ) -> Result<WeatherSnapshot, PluginWeatherError> {
        let runtime = self.weather_runtime(WeatherCapability::AirportReports)?;
        let request = WeatherRequest {
            id: self.next_weather_request_id(&runtime)?,
            query: WeatherQuery::AirportReports {
                stations: stations.to_vec(),
                window,
            },
        };
        request
            .validate()
            .map_err(|_| PluginWeatherError::InvalidResponse)?;
        let request_id = request.id.clone();
        let (sender, receiver) = mpsc::channel();
        self.queue_weather_request(&runtime, request, Some(sender))
            .map_err(plugin_weather_service_error)?;

        let response = match receiver.recv_timeout(AIRPORT_WEATHER_RESPONSE_TIMEOUT) {
            Ok(response) => response,
            Err(_) => {
                if let Ok(mut pending) = runtime.pending_weather.lock() {
                    pending.remove(&request_id);
                }
                return Err(PluginWeatherError::TimedOut);
            }
        };
        match response {
            PluginWeatherResponse::Complete { product } => match product {
                PluginWeatherProduct::AirportReports { snapshot } => Ok(snapshot),
                _ => Err(PluginWeatherError::InvalidResponse),
            },
            PluginWeatherResponse::Unavailable { code } => Err(weather_unavailable_error(code)),
        }
    }

    pub fn request_historical_global_weather(
        &self,
        window: WeatherTimeWindow,
    ) -> Result<GlobalWeatherLayerSnapshot, PluginWeatherError> {
        let runtime = {
            let runtimes = self
                .inner
                .runtimes
                .lock()
                .map_err(|_| PluginWeatherError::StateUnavailable)?;
            runtimes
                .get(OPEN_METEO_PLUGIN_ID)
                .filter(|runtime| {
                    runtime
                        .weather_capabilities
                        .contains(&WeatherCapability::ForecastGrid)
                        && runtime_state(runtime) == Ok(PluginProcessState::Running)
                })
                .cloned()
                .ok_or(PluginWeatherError::ProviderUnavailable)?
        };
        let WeatherQuery::ForecastGrid { points, .. } = default_global_weather_grid() else {
            unreachable!("the default model query is always a grid")
        };
        let request = WeatherRequest {
            id: self.next_weather_request_id(&runtime)?,
            query: WeatherQuery::ForecastGrid {
                points,
                window: Some(window),
            },
        };
        request
            .validate()
            .map_err(|_| PluginWeatherError::InvalidResponse)?;
        let request_id = request.id.clone();
        let (sender, receiver) = mpsc::channel();
        self.queue_weather_request(&runtime, request, Some(sender))
            .map_err(plugin_weather_service_error)?;
        let response = match receiver.recv_timeout(GLOBAL_WEATHER_RESPONSE_TIMEOUT) {
            Ok(response) => response,
            Err(_) => {
                if let Ok(mut pending) = runtime.pending_weather.lock() {
                    pending.remove(&request_id);
                }
                return Err(PluginWeatherError::TimedOut);
            }
        };
        match response {
            PluginWeatherResponse::Complete { product } => match product {
                PluginWeatherProduct::GlobalLayer { layer } => Ok(layer),
                _ => Err(PluginWeatherError::InvalidResponse),
            },
            PluginWeatherResponse::Unavailable { code } => Err(weather_unavailable_error(code)),
        }
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
        let requested = requested_capability_names(manifest);
        let subject = AuthorizationSubject::plugin(&manifest.id);
        let revision = plugin_scope_revision(manifest);
        self.inner
            .authorization
            .grants(&subject, &revision, &requested)
            .map_err(plugin_authorization_error)
            .map(capability_names_to_permissions)
    }

    fn start_with_wyrmgrid(&self, manifest: &PluginManifest) -> Result<bool, PluginError> {
        self.inner
            .preferences
            .load_plugin_preferences_record(&manifest.id)
            .map_err(|_| PluginError::StorageUnavailable)
            .map(|record| {
                record.is_some_and(|record| {
                    record.start_with_wyrmgrid
                        && record.scope_revision == plugin_scope_revision(manifest)
                })
            })
    }

    fn configuration_for(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Vec<PluginSettingView>, PluginError> {
        plugin_setting_definitions(manifest)
            .into_iter()
            .map(|definition| {
                let value = self.configured_minutes(manifest, definition)?;
                Ok(PluginSettingView {
                    key: definition.key.to_owned(),
                    value: value.to_string(),
                    choices: definition
                        .options
                        .iter()
                        .map(|minutes| PluginSettingChoice {
                            value: minutes.to_string(),
                        })
                        .collect(),
                })
            })
            .collect()
    }

    fn weather_refresh_intervals(
        &self,
        manifest: &PluginManifest,
    ) -> Result<BTreeMap<WeatherCapability, Duration>, PluginError> {
        plugin_setting_definitions(manifest)
            .into_iter()
            .map(|definition| {
                self.configured_minutes(manifest, definition)
                    .map(|minutes| {
                        (
                            definition.capability,
                            Duration::from_secs(u64::from(minutes) * 60),
                        )
                    })
            })
            .collect()
    }

    fn configured_minutes(
        &self,
        manifest: &PluginManifest,
        definition: PluginSettingDefinition,
    ) -> Result<u16, PluginError> {
        let configured = self
            .inner
            .preferences
            .load_plugin_configuration_record(&manifest.id, definition.key)
            .map_err(|_| PluginError::StorageUnavailable)?
            .and_then(|record| record.value.parse::<u16>().ok())
            .filter(|value| definition.options.contains(value));
        Ok(configured.unwrap_or(definition.default_minutes))
    }

    fn clear_startup_failure(&self, plugin_id: &str) -> Result<(), PluginError> {
        self.inner
            .startup_failures
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .remove(plugin_id);
        Ok(())
    }

    fn grant_lifetime_for(
        &self,
        manifest: &PluginManifest,
    ) -> Result<Option<AuthorizationGrantLifetime>, PluginError> {
        let requested = requested_capability_names(manifest);
        let subject = AuthorizationSubject::plugin(&manifest.id);
        let revision = plugin_scope_revision(manifest);
        self.inner
            .authorization
            .grant_lifetime(&subject, &revision, &requested)
            .map_err(plugin_authorization_error)
    }

    fn required_grants_for(
        &self,
        manifest: &PluginManifest,
    ) -> Result<BTreeSet<Permission>, PluginError> {
        let requested = requested_capability_names(manifest);
        let subject = AuthorizationSubject::plugin(&manifest.id);
        let revision = plugin_scope_revision(manifest);
        self.inner
            .authorization
            .require_all(&subject, &revision, &requested)
            .map_err(plugin_authorization_error)
            .map(capability_names_to_permissions)
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
            fail_runtime(runtime, PluginRuntimeFailure::ProcessStopped);
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

    fn weather_runtime(
        &self,
        capability: WeatherCapability,
    ) -> Result<Arc<RunningPlugin>, PluginWeatherError> {
        let runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| PluginWeatherError::StateUnavailable)?;
        runtimes
            .values()
            .find(|runtime| {
                runtime.weather_capabilities.contains(&capability)
                    && runtime_state(runtime) == Ok(PluginProcessState::Running)
            })
            .cloned()
            .ok_or(PluginWeatherError::ProviderUnavailable)
    }

    fn next_weather_request_id(
        &self,
        runtime: &Arc<RunningPlugin>,
    ) -> Result<String, PluginWeatherError> {
        let mut sequence = runtime
            .weather_request_sequence
            .lock()
            .map_err(|_| PluginWeatherError::StateUnavailable)?;
        let request_id = format!("weather-{sequence}");
        *sequence = sequence.saturating_add(1);
        Ok(request_id)
    }

    fn queue_weather_request(
        &self,
        runtime: &Arc<RunningPlugin>,
        request: WeatherRequest,
        response_sender: Option<Sender<PluginWeatherResponse>>,
    ) -> Result<(), PluginError> {
        request
            .validate()
            .map_err(|_| PluginError::ProtocolViolation)?;
        let capability = request.query.capability();
        if !runtime
            .granted_permissions
            .contains(&Permission::WeatherDataPublish)
            || !runtime.weather_capabilities.contains(&capability)
        {
            return Err(PluginError::UnsupportedCapability);
        }
        let request_id = request.id.clone();
        {
            let mut pending = runtime
                .pending_weather
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?;
            if pending.contains_key(&request_id) {
                return Err(PluginError::ProtocolViolation);
            }
            pending.insert(
                request_id.clone(),
                PendingWeatherRequest {
                    capability,
                    query: request.query.clone(),
                    queued_at: Instant::now(),
                    response_sender,
                },
            );
        }
        if let Err(error) = self.send_message(runtime, HostMessage::WeatherRequest { request }) {
            if let Ok(mut pending) = runtime.pending_weather.lock() {
                pending.remove(&request_id);
            }
            return Err(error);
        }
        Ok(())
    }

    fn send_global_weather_if_due(&self, runtime: &Arc<RunningPlugin>) -> Result<(), PluginError> {
        let weather_request_expired = runtime
            .pending_weather
            .lock()
            .map_err(|_| PluginError::StateUnavailable)?
            .values()
            .any(|pending| pending.queued_at.elapsed() >= GLOBAL_WEATHER_RESPONSE_TIMEOUT);
        if weather_request_expired {
            fail_runtime(runtime, PluginRuntimeFailure::WeatherRequestTimedOut);
            return Err(PluginError::ProtocolViolation);
        }
        for capability in [
            WeatherCapability::ForecastGrid,
            WeatherCapability::RadarTiles,
        ] {
            if !runtime.weather_capabilities.contains(&capability) {
                continue;
            }
            let has_pending = runtime
                .pending_weather
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?
                .values()
                .any(|pending| pending.capability == capability);
            if has_pending {
                continue;
            }
            let refresh_interval = runtime
                .weather_refresh_intervals
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?
                .get(&capability)
                .copied()
                .unwrap_or_else(|| default_weather_refresh_interval(capability));
            let due = runtime
                .last_weather_requests
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?
                .get(&capability)
                .is_none_or(|last_request| last_request.elapsed() >= refresh_interval);
            if !due {
                continue;
            }
            let queries = match capability {
                WeatherCapability::ForecastGrid => vec![default_global_weather_grid()],
                WeatherCapability::RadarTiles if runtime.plugin_id == RAINVIEWER_PLUGIN_ID => (0
                    ..=wyrmgrid_plugin_protocol::MAX_RADAR_FRAME_OFFSET)
                    .rev()
                    .map(|offset| default_global_radar_tiles(Some(offset)))
                    .collect(),
                WeatherCapability::RadarTiles => vec![default_global_radar_tiles(None)],
                WeatherCapability::AirportReports => continue,
            };
            for query in queries {
                let request_id = self
                    .next_weather_request_id(runtime)
                    .map_err(plugin_weather_error_to_plugin_error)?;
                self.queue_weather_request(
                    runtime,
                    WeatherRequest {
                        id: request_id,
                        query,
                    },
                    None,
                )?;
            }
            runtime
                .last_weather_requests
                .lock()
                .map_err(|_| PluginError::StateUnavailable)?
                .insert(capability, Instant::now());
        }
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
                    || service.send_global_weather_if_due(&runtime).is_err()
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
    install_bundled_python_plugin(
        root,
        BUNDLED_PLUGIN_ID,
        BUNDLED_MANIFEST,
        BUNDLED_ENTRY_POINT,
    )?;
    install_bundled_python_plugin(
        root,
        OPEN_METEO_PLUGIN_ID,
        OPEN_METEO_MANIFEST,
        OPEN_METEO_ENTRY_POINT,
    )?;
    install_bundled_python_plugin(
        root,
        AVIATION_WEATHER_PLUGIN_ID,
        AVIATION_WEATHER_MANIFEST,
        AVIATION_WEATHER_ENTRY_POINT,
    )?;
    install_bundled_python_plugin(
        root,
        RAINVIEWER_PLUGIN_ID,
        RAINVIEWER_MANIFEST,
        RAINVIEWER_ENTRY_POINT,
    )
}

fn install_bundled_python_plugin(
    root: &Path,
    plugin_id: &str,
    manifest: &str,
    entry_point: &str,
) -> Result<(), io::Error> {
    let plugin_root = root.join(plugin_id);
    let source_root = plugin_root.join("src");
    let sdk_root = source_root.join("wyrmgrid_sdk");
    fs::create_dir_all(&sdk_root)?;
    write_bundled_file(&plugin_root.join("plugin.json"), manifest)?;
    write_bundled_file(&source_root.join("main.py"), entry_point)?;
    write_bundled_file(&sdk_root.join("__init__.py"), BUNDLED_PYTHON_SDK)
}

fn write_bundled_file(path: &Path, contents: &str) -> Result<(), io::Error> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == contents => {}
        Ok(_) => fs::write(path, contents)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::write(path, contents)?,
        Err(error) => return Err(error),
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
    let supported = permissions.iter().all(|permission| {
        matches!(
            permission,
            Permission::OnAirFleetRead
                | Permission::MapLayersPublish
                | Permission::SimulatorTelemetryRead
                | Permission::ExternalNetwork
                | Permission::WeatherDataPublish
        )
    });
    let weather_network_pair = permissions.contains(&Permission::ExternalNetwork)
        == permissions.contains(&Permission::WeatherDataPublish);
    (supported && weather_network_pair)
        .then_some(())
        .ok_or(PluginError::UnsupportedCapability)
}

fn requested_capability_names(manifest: &PluginManifest) -> BTreeSet<String> {
    manifest
        .permissions
        .iter()
        .map(|permission| permission.as_str().to_owned())
        .collect()
}

fn capability_names_to_permissions(capabilities: BTreeSet<String>) -> BTreeSet<Permission> {
    capabilities
        .iter()
        .filter_map(|capability| Permission::from_name(capability))
        .collect()
}

fn plugin_setting_definitions(manifest: &PluginManifest) -> Vec<PluginSettingDefinition> {
    let mut definitions = Vec::new();
    if manifest
        .weather_capabilities
        .contains(&WeatherCapability::ForecastGrid)
    {
        definitions.push(PluginSettingDefinition {
            key: FORECAST_REFRESH_SETTING_KEY,
            capability: WeatherCapability::ForecastGrid,
            default_minutes: 15,
            options: &FORECAST_REFRESH_OPTIONS,
        });
    }
    if manifest
        .weather_capabilities
        .contains(&WeatherCapability::RadarTiles)
    {
        definitions.push(PluginSettingDefinition {
            key: RADAR_REFRESH_SETTING_KEY,
            capability: WeatherCapability::RadarTiles,
            default_minutes: 5,
            options: &RADAR_REFRESH_OPTIONS,
        });
    }
    definitions
}

fn plugin_setting_definition(
    manifest: &PluginManifest,
    key: &str,
) -> Option<PluginSettingDefinition> {
    plugin_setting_definitions(manifest)
        .into_iter()
        .find(|definition| definition.key == key)
}

fn default_weather_refresh_interval(capability: WeatherCapability) -> Duration {
    match capability {
        WeatherCapability::ForecastGrid => MODEL_WEATHER_REFRESH_INTERVAL,
        WeatherCapability::RadarTiles => RADAR_WEATHER_REFRESH_INTERVAL,
        WeatherCapability::AirportReports => MODEL_WEATHER_REFRESH_INTERVAL,
    }
}

fn plugin_scope_revision(manifest: &PluginManifest) -> String {
    let capabilities = requested_capability_names(manifest);
    let capability_revision = if capabilities.is_empty() {
        "none".to_owned()
    } else {
        capabilities.into_iter().collect::<Vec<_>>().join("|")
    };
    let weather_revision = if manifest.weather_capabilities.is_empty() {
        "none".to_owned()
    } else {
        manifest
            .weather_capabilities
            .iter()
            .map(|capability| capability.as_str())
            .collect::<Vec<_>>()
            .join("|")
    };
    let network_revision = if manifest.network_origins.is_empty() {
        "none".to_owned()
    } else {
        manifest.network_origins.join("|")
    };
    format!(
        "plugin:{}:{capability_revision}:weather={weather_revision}:network={network_revision}",
        manifest.version
    )
}

fn default_global_weather_grid() -> WeatherQuery {
    let latitudes = [-75.0, -50.0, -25.0, 0.0, 25.0, 50.0, 75.0];
    let longitudes = [
        -165.0, -135.0, -105.0, -75.0, -45.0, -15.0, 15.0, 45.0, 75.0, 105.0, 135.0, 165.0,
    ];
    let mut points = Vec::with_capacity(latitudes.len() * longitudes.len());
    for (row, latitude) in latitudes.into_iter().enumerate() {
        for (column, longitude) in longitudes.into_iter().enumerate() {
            points.push(WeatherGridRequestPoint {
                id: format!("global-{row}-{column}"),
                location: wyrmgrid_domain::Coordinates {
                    latitude,
                    longitude,
                },
            });
        }
    }
    WeatherQuery::ForecastGrid {
        points,
        window: None,
    }
}

fn default_global_radar_tiles(frame_offset: Option<u8>) -> WeatherQuery {
    WeatherQuery::RadarTiles {
        tiles: vec![
            WeatherTileAddress {
                zoom: 1,
                x: 0,
                y: 0,
            },
            WeatherTileAddress {
                zoom: 1,
                x: 1,
                y: 0,
            },
            WeatherTileAddress {
                zoom: 1,
                x: 0,
                y: 1,
            },
            WeatherTileAddress {
                zoom: 1,
                x: 1,
                y: 1,
            },
        ],
        frame_offset,
    }
}

fn weather_unavailable_error(code: WeatherUnavailableCode) -> PluginWeatherError {
    match code {
        WeatherUnavailableCode::Offline => PluginWeatherError::Offline,
        WeatherUnavailableCode::TimedOut => PluginWeatherError::TimedOut,
        WeatherUnavailableCode::RateLimited => PluginWeatherError::RateLimited,
        WeatherUnavailableCode::ProviderUnavailable => PluginWeatherError::ProviderUnavailable,
        WeatherUnavailableCode::InvalidResponse => PluginWeatherError::InvalidResponse,
        WeatherUnavailableCode::NoData => PluginWeatherError::NoData,
    }
}

fn plugin_weather_service_error(error: PluginError) -> PluginWeatherError {
    match error {
        PluginError::StateUnavailable | PluginError::StorageUnavailable => {
            PluginWeatherError::StateUnavailable
        }
        PluginError::ProtocolViolation => PluginWeatherError::InvalidResponse,
        _ => PluginWeatherError::ProviderUnavailable,
    }
}

fn plugin_weather_error_to_plugin_error(error: PluginWeatherError) -> PluginError {
    match error {
        PluginWeatherError::StateUnavailable => PluginError::StateUnavailable,
        PluginWeatherError::InvalidResponse => PluginError::ProtocolViolation,
        _ => PluginError::UnsupportedCapability,
    }
}

fn plugin_authorization_error(error: AuthorizationError) -> PluginError {
    match error {
        AuthorizationError::StorageUnavailable => PluginError::StorageUnavailable,
        AuthorizationError::CapabilityRequired => PluginError::PermissionRequired,
        AuthorizationError::InvalidCapability => PluginError::UnsupportedCapability,
        AuthorizationError::InvalidSubject | AuthorizationError::InvalidScopeRevision => {
            PluginError::InvalidPlugin
        }
    }
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
    weather_capabilities: BTreeSet<WeatherCapability>,
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
                        fail_runtime(&runtime, PluginRuntimeFailure::ProcessStopped);
                    }
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            };
            if envelope.validate_header().is_err() || envelope.sequence <= last_sequence {
                fail_runtime(&runtime, PluginRuntimeFailure::ProtocolEnvelopeInvalid);
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
                        fail_runtime(&runtime, PluginRuntimeFailure::MapLayerInvalid);
                        return;
                    }
                    let mut layers = match runtime.layers.lock() {
                        Ok(layers) => layers,
                        Err(_) => {
                            fail_runtime(&runtime, PluginRuntimeFailure::SupervisorUnavailable);
                            return;
                        }
                    };
                    if !layers.contains_key(&layer.id) && layers.len() >= MAX_MAP_LAYERS_PER_PLUGIN
                    {
                        drop(layers);
                        fail_runtime(&runtime, PluginRuntimeFailure::MapLayerLimitExceeded);
                        return;
                    }
                    layers.insert(layer.id.clone(), layer);
                }
                PluginMessage::PublishWeather {
                    request_id,
                    response,
                } if ready && granted_permissions.contains(&Permission::WeatherDataPublish) => {
                    let pending = match runtime.pending_weather.lock() {
                        Ok(mut pending) => pending.remove(&request_id),
                        Err(_) => {
                            fail_runtime(&runtime, PluginRuntimeFailure::SupervisorUnavailable);
                            return;
                        }
                    };
                    let Some(pending) = pending else {
                        fail_runtime(&runtime, PluginRuntimeFailure::WeatherRequestUnknown);
                        return;
                    };
                    if !weather_capabilities.contains(&pending.capability) {
                        fail_runtime(&runtime, PluginRuntimeFailure::WeatherCapabilityUndeclared);
                        return;
                    }
                    if let Err(failure) =
                        validate_weather_response_for_request(&response, &pending.query)
                    {
                        fail_runtime(&runtime, failure);
                        return;
                    }
                    if let PluginWeatherResponse::Complete { product } = &response
                        && let PluginWeatherProduct::GlobalLayer { layer } = product
                    {
                        let mut layers = match runtime.weather_layers.lock() {
                            Ok(layers) => layers,
                            Err(_) => {
                                fail_runtime(&runtime, PluginRuntimeFailure::SupervisorUnavailable);
                                return;
                            }
                        };
                        if !layers.contains_key(&layer.id)
                            && layers.len() >= MAX_WEATHER_LAYERS_PER_PLUGIN
                        {
                            drop(layers);
                            fail_runtime(&runtime, PluginRuntimeFailure::WeatherLayerLimitExceeded);
                            return;
                        }
                        insert_weather_layer(&mut layers, layer.clone());
                    }
                    if let Some(sender) = pending.response_sender {
                        let _ = sender.send(response);
                    }
                }
                _ => {
                    fail_runtime(&runtime, PluginRuntimeFailure::MessageUnauthorized);
                    if let Some(sender) = ready_sender.take() {
                        let _ = sender.send(false);
                    }
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
fn weather_response_matches_request(
    response: &PluginWeatherResponse,
    query: &WeatherQuery,
) -> bool {
    validate_weather_response_for_request(response, query).is_ok()
}

fn validate_weather_response_for_request(
    response: &PluginWeatherResponse,
    query: &WeatherQuery,
) -> Result<(), PluginRuntimeFailure> {
    let PluginWeatherResponse::Complete { product } = response else {
        return Ok(());
    };
    if !product.validate() {
        return Err(PluginRuntimeFailure::WeatherProductInvalid);
    }
    if product.capability() != query.capability() {
        return Err(PluginRuntimeFailure::WeatherProductCapabilityMismatch);
    }
    let matches_request = match (product, query) {
        (
            PluginWeatherProduct::AirportReports { snapshot },
            WeatherQuery::AirportReports { stations, window },
        ) => {
            let mut actual = snapshot
                .airports
                .iter()
                .map(|airport| airport.station_icao.as_str())
                .collect::<Vec<_>>();
            let mut expected = stations.iter().map(String::as_str).collect::<Vec<_>>();
            actual.sort_unstable();
            expected.sort_unstable();
            actual == expected && airport_reports_match_window(snapshot, window.as_ref())
        }
        (
            PluginWeatherProduct::GlobalLayer { layer },
            WeatherQuery::ForecastGrid { points, window },
        ) => {
            let wyrmgrid_domain::GlobalWeatherLayerData::Grid {
                points: actual_points,
            } = &layer.data
            else {
                return Err(PluginRuntimeFailure::WeatherProductRequestMismatch);
            };
            forecast_points_match_request(actual_points, points)
                && forecast_layer_matches_window(layer, window.as_ref())
        }
        (PluginWeatherProduct::GlobalLayer { layer }, WeatherQuery::RadarTiles { tiles, .. }) => {
            let wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles {
                tiles: actual_tiles,
                ..
            } = &layer.data
            else {
                return Err(PluginRuntimeFailure::WeatherProductRequestMismatch);
            };
            actual_tiles.len() == tiles.len()
                && tiles.iter().all(|expected| {
                    actual_tiles.iter().any(|actual| {
                        actual.zoom == expected.zoom
                            && actual.x == expected.x
                            && actual.y == expected.y
                    })
                })
        }
        _ => false,
    };
    if matches_request {
        Ok(())
    } else {
        Err(PluginRuntimeFailure::WeatherProductRequestMismatch)
    }
}

fn airport_reports_match_window(
    snapshot: &WeatherSnapshot,
    window: Option<&WeatherTimeWindow>,
) -> bool {
    let Some(window) = window else {
        return true;
    };
    snapshot.airports.iter().all(|airport| {
        airport.taf.is_none()
            && airport.metar.as_ref().is_none_or(|metar| {
                (window.starts_at..=window.ends_at).contains(&metar.value.observed_at)
            })
    })
}

fn forecast_layer_matches_window(
    layer: &GlobalWeatherLayerSnapshot,
    window: Option<&WeatherTimeWindow>,
) -> bool {
    match window {
        None => layer
            .time_scope
            .as_ref()
            .is_none_or(|scope| scope.kind == GlobalWeatherTimeKind::CurrentForecast),
        Some(window) => {
            let Some(scope) = layer.time_scope.as_ref() else {
                return false;
            };
            scope.kind == GlobalWeatherTimeKind::HistoricalModel
                && scope.target_at == window.target_at
                && scope.starts_at == window.starts_at
                && scope.ends_at == window.ends_at
                && match &layer.data {
                    wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } => {
                        points.iter().all(|point| {
                            point.valid_at.is_some_and(|valid_at| {
                                (window.starts_at..=window.ends_at).contains(&valid_at)
                            })
                        })
                    }
                    wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles { .. } => false,
                }
        }
    }
}

fn forecast_points_match_request(
    actual_points: &[wyrmgrid_domain::GlobalWeatherGridPoint],
    requested_points: &[WeatherGridRequestPoint],
) -> bool {
    requested_points.iter().all(|requested| {
        actual_points
            .iter()
            .any(|actual| forecast_point_matches_request(actual, requested))
    }) && actual_points.iter().all(|actual| {
        requested_points
            .iter()
            .filter(|requested| forecast_point_matches_request(actual, requested))
            .count()
            == 1
    })
}

fn forecast_point_matches_request(
    actual: &wyrmgrid_domain::GlobalWeatherGridPoint,
    requested: &WeatherGridRequestPoint,
) -> bool {
    if actual.location != requested.location {
        return false;
    }
    if actual.id == requested.id {
        return true;
    }
    actual.valid_at.is_some()
        && actual
            .id
            .strip_prefix(&requested.id)
            .and_then(|suffix| suffix.strip_prefix("-h"))
            .is_some_and(|horizon| {
                horizon.len() == 2 && horizon.bytes().all(|byte| byte.is_ascii_digit())
            })
}

fn insert_weather_layer(
    layers: &mut BTreeMap<String, Vec<GlobalWeatherLayerSnapshot>>,
    layer: GlobalWeatherLayerSnapshot,
) {
    let layer_id = layer.id.clone();
    let history = layers.entry(layer_id).or_default();
    let wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles { frame_time, .. } = &layer.data
    else {
        history.clear();
        history.push(layer);
        return;
    };
    if let Some(existing) = history.iter_mut().find(|candidate| {
        matches!(
            &candidate.data,
            wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles {
                frame_time: candidate_time,
                ..
            } if candidate_time == frame_time
        )
    }) {
        *existing = layer;
    } else {
        history.push(layer);
    }
    history.sort_by_key(|candidate| match &candidate.data {
        wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles { frame_time, .. } => *frame_time,
        wyrmgrid_domain::GlobalWeatherLayerData::Grid { .. } => {
            chrono::DateTime::<chrono::Utc>::MIN_UTC
        }
    });
    if history.len() > MAX_RADAR_FRAMES_PER_LAYER {
        history.drain(..history.len() - MAX_RADAR_FRAMES_PER_LAYER);
    }
}

fn runtime_state(runtime: &Arc<RunningPlugin>) -> Result<PluginProcessState, PluginError> {
    runtime
        .state
        .lock()
        .map(|state| *state)
        .map_err(|_| PluginError::StateUnavailable)
}

fn fail_runtime(runtime: &Arc<RunningPlugin>, failure: PluginRuntimeFailure) {
    let diagnostic = failure.diagnostic(&runtime.plugin_id);
    if !mark_plugin_failed(&runtime.state, &runtime.failure_recorded) {
        return;
    }
    if let Ok(mut last_error) = runtime.last_error.lock() {
        *last_error = Some(diagnostic.message.to_owned());
    }
    if let Ok(mut layers) = runtime.layers.lock() {
        layers.clear();
    }
    if let Ok(mut layers) = runtime.weather_layers.lock() {
        layers.clear();
    }
    notify_pending_weather(runtime, WeatherUnavailableCode::ProviderUnavailable);
    if let Some(observer) = runtime.diagnostic_observer.as_ref() {
        observer.observe(&diagnostic);
    }
    if let Ok(mut child) = runtime.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn mark_plugin_failed(state: &Mutex<PluginProcessState>, failure_recorded: &AtomicBool) -> bool {
    if failure_recorded.swap(true, Ordering::AcqRel) {
        return false;
    }
    if let Ok(mut state) = state.lock() {
        *state = PluginProcessState::Failed;
    }
    true
}

fn notify_pending_weather(runtime: &Arc<RunningPlugin>, code: WeatherUnavailableCode) {
    let pending = runtime
        .pending_weather
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default();
    for request in pending.into_values() {
        if let Some(sender) = request.response_sender {
            let _ = sender.send(PluginWeatherResponse::Unavailable { code });
        }
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

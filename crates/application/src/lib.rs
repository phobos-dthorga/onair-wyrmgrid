//! Application-level orchestration independent of Tauri and other interfaces.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{AircraftSummary, CompanyId, CompanySummary, FboSummary, Observed};
use wyrmgrid_onair_api::{ClientError, DEFAULT_BASE_URL, OnAirClient};
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;
use wyrmgrid_storage::Store;

const FLEET_RESOURCE_KIND: &str = "onair_company_fleet";
const FLEET_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const FBOS_RESOURCE_KIND: &str = "onair_company_fbos";
const FBOS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformStatus {
    pub application: &'static str,
    pub version: &'static str,
    pub plugin_api_version: u32,
    pub mode: &'static str,
}

pub fn platform_status() -> PlatformStatus {
    PlatformStatus {
        application: "OnAir WyrmGrid",
        version: env!("CARGO_PKG_VERSION"),
        plugin_api_version: PLUGIN_API_VERSION,
        mode: "foundation",
    }
}

pub const TERMS_VERSION: &str = "2026-07-14";
pub const PRIVACY_NOTICE_VERSION: &str = "2026-07-14";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalPreferences {
    pub terms_version: String,
    pub privacy_notice_version: String,
    pub telemetry_enabled: bool,
    pub acknowledged_at: String,
}

pub trait LegalPreferencesRepository: Send + Sync + 'static {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError>;

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError>;
}

impl LegalPreferencesRepository for Store {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
        self.load_legal_preferences_record()
            .map(|preferences| {
                preferences.map(|preferences| PersistedLegalPreferences {
                    terms_version: preferences.terms_version,
                    privacy_notice_version: preferences.privacy_notice_version,
                    telemetry_enabled: preferences.telemetry_enabled,
                    acknowledged_at: preferences.acknowledged_at,
                })
            })
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError> {
        self.save_legal_preferences_record(terms_version, privacy_notice_version, telemetry_enabled)
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegalStatus {
    pub terms_version: &'static str,
    pub privacy_notice_version: &'static str,
    pub acknowledged: bool,
    pub telemetry_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<String>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LegalSettingsError {
    #[error("WyrmGrid could not read or save its local privacy preferences.")]
    StorageUnavailable,
    #[error("Review the current Terms and Privacy Notice before changing this preference.")]
    AcknowledgementRequired,
}

pub struct LegalSettingsService<R> {
    repository: R,
}

impl<R: LegalPreferencesRepository> LegalSettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<LegalStatus, LegalSettingsError> {
        let stored = self.repository.load_legal_preferences()?;
        let acknowledged = stored.as_ref().is_some_and(|preferences| {
            preferences.terms_version == TERMS_VERSION
                && preferences.privacy_notice_version == PRIVACY_NOTICE_VERSION
        });
        let acknowledged_at = if acknowledged {
            stored
                .as_ref()
                .map(|preferences| preferences.acknowledged_at.clone())
        } else {
            None
        };

        Ok(LegalStatus {
            terms_version: TERMS_VERSION,
            privacy_notice_version: PRIVACY_NOTICE_VERSION,
            acknowledged,
            telemetry_enabled: acknowledged
                && stored
                    .as_ref()
                    .is_some_and(|preferences| preferences.telemetry_enabled),
            acknowledged_at,
        })
    }

    pub fn acknowledge(&self, telemetry_enabled: bool) -> Result<LegalStatus, LegalSettingsError> {
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }

    pub fn update_telemetry(
        &self,
        telemetry_enabled: bool,
    ) -> Result<LegalStatus, LegalSettingsError> {
        if !self.status()?.acknowledged {
            return Err(LegalSettingsError::AcknowledgementRequired);
        }
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub company: Option<ConnectedCompany>,
    pub credential_storage: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectedCompany {
    pub name: String,
    pub airline_code: String,
}

impl From<&CompanySummary> for ConnectedCompany {
    fn from(company: &CompanySummary) -> Self {
        Self {
            name: company.name.clone(),
            airline_code: company.airline_code.clone(),
        }
    }
}

pub const MANUAL_SYNC_COOLDOWN: Duration = Duration::from_secs(60);
pub const MINIMUM_AUTOMATIC_SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSyncTrigger {
    Initial,
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSyncDisposition {
    Synchronized,
    QuietlyIgnored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotAvailability {
    Live,
    Cached,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStorage {
    Hoard,
    MemoryOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<Vec<AircraftSummary>>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FboSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<Vec<FboSummary>>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyDataResource {
    Fleet,
    Fbos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataSyncFailure {
    pub resource: CompanyDataResource,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompanyDataSyncResult {
    pub disposition: DataSyncDisposition,
    pub fleet: Option<FleetSnapshotView>,
    pub fbos: Option<FboSnapshotView>,
    pub failures: Vec<DataSyncFailure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredFleetSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<Vec<AircraftSummary>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredFboSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<Vec<FboSummary>>,
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Enter a valid OnAir company ID.")]
    InvalidCompanyId,
    #[error("Enter your OnAir API key.")]
    EmptyApiKey,
    #[error(
        "OnAir rejected these details. For now, copy them from OnAir Client → Options → Global Settings—not OnAir Companion."
    )]
    AuthenticationRejected,
    #[error("That company was not found in the selected OnAir world.")]
    CompanyNotFound,
    #[error("OnAir is receiving too many requests. Please wait before trying again.")]
    RateLimited,
    #[error("WyrmGrid could not reach OnAir. Check your connection and try again.")]
    ServiceUnavailable,
    #[error("The local connection state is unavailable.")]
    StateUnavailable,
    #[error("Connect to OnAir before synchronizing company data.")]
    NotConnected,
    #[error(
        "WyrmGrid could not refresh the fleet. A previous successful observation, if present, remains available."
    )]
    FleetUnavailable,
    #[error(
        "WyrmGrid could not refresh the FBO network. A previous successful observation, if present, remains available."
    )]
    FbosUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub reportable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_id: Option<String>,
}

impl OperationError {
    pub fn with_report_id(mut self, report_id: Option<String>) -> Self {
        self.report_id = report_id;
        self
    }
}

impl From<ConnectionError> for OperationError {
    fn from(error: ConnectionError) -> Self {
        let (code, retryable, reportable) = match &error {
            ConnectionError::InvalidCompanyId => ("onair.invalid_company_id", false, false),
            ConnectionError::EmptyApiKey => ("onair.empty_api_key", false, false),
            ConnectionError::AuthenticationRejected => {
                ("onair.authentication_rejected", false, false)
            }
            ConnectionError::CompanyNotFound => ("onair.company_not_found", false, false),
            ConnectionError::RateLimited => ("onair.rate_limited", true, false),
            ConnectionError::ServiceUnavailable => ("onair.service_unavailable", true, false),
            ConnectionError::StateUnavailable => ("application.state_unavailable", true, true),
            ConnectionError::NotConnected => ("onair.not_connected", false, false),
            ConnectionError::FleetUnavailable => ("onair.fleet_unavailable", true, false),
            ConnectionError::FbosUnavailable => ("onair.fbos_unavailable", true, false),
        };

        Self {
            code,
            message: error.to_string(),
            retryable,
            reportable,
            report_id: None,
        }
    }
}

impl From<LegalSettingsError> for OperationError {
    fn from(error: LegalSettingsError) -> Self {
        let code = match error {
            LegalSettingsError::StorageUnavailable => "legal.storage_unavailable",
            LegalSettingsError::AcknowledgementRequired => "legal.acknowledgement_required",
        };
        Self {
            code,
            message: error.to_string(),
            retryable: matches!(error, LegalSettingsError::StorageUnavailable),
            reportable: false,
            report_id: None,
        }
    }
}

#[derive(Clone)]
pub struct OnAirSession {
    inner: Arc<RwLock<Option<ConnectedSession>>>,
    fleet: Arc<RwLock<Option<FleetSnapshotView>>>,
    fbos: Arc<RwLock<Option<FboSnapshotView>>>,
    store: Arc<Mutex<Store>>,
    base_url: &'static str,
}

struct ConnectedSession {
    client: Arc<OnAirClient>,
    company: CompanySummary,
    data_sync_gate: Arc<Mutex<DataSyncGate>>,
}

#[derive(Debug, Default)]
struct DataSyncGate {
    in_progress: bool,
    last_started: Option<Instant>,
}

impl DataSyncGate {
    fn try_start(&mut self, trigger: DataSyncTrigger, now: Instant) -> bool {
        if self.in_progress {
            return false;
        }

        let minimum_interval = match trigger {
            DataSyncTrigger::Initial => Duration::ZERO,
            DataSyncTrigger::Manual => MANUAL_SYNC_COOLDOWN,
            DataSyncTrigger::Automatic => MINIMUM_AUTOMATIC_SYNC_INTERVAL,
        };
        if self
            .last_started
            .is_some_and(|last_started| now.duration_since(last_started) < minimum_interval)
        {
            return false;
        }

        self.in_progress = true;
        self.last_started = Some(now);
        true
    }

    fn finish(&mut self) {
        self.in_progress = false;
    }
}

struct DataSyncPermit {
    gate: Arc<Mutex<DataSyncGate>>,
}

impl Drop for DataSyncPermit {
    fn drop(&mut self) {
        if let Ok(mut gate) = self.gate.lock() {
            gate.finish();
        }
    }
}

impl Default for OnAirSession {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL)
    }
}

impl OnAirSession {
    pub fn new(base_url: &'static str) -> Self {
        let store = Store::open_in_memory().expect("in-memory Hoard should initialize");
        Self::with_store(base_url, store)
    }

    pub fn with_store(base_url: &'static str, store: Store) -> Self {
        let persistent = store.is_persistent();
        let stored_fleet = load_stored_fleet(&store, None);
        let anchor_company = stored_fleet
            .as_ref()
            .map(|stored| stored.company.id.clone());
        let stored_fbos = load_stored_fbos(&store, anchor_company.as_ref());
        let storage = if persistent {
            SnapshotStorage::Hoard
        } else {
            SnapshotStorage::MemoryOnly
        };
        let cached_fleet =
            stored_fleet.map(|stored| fleet_view(stored, SnapshotAvailability::Offline, storage));
        let cached_fbos =
            stored_fbos.map(|stored| fbo_view(stored, SnapshotAvailability::Offline, storage));
        Self {
            inner: Arc::new(RwLock::new(None)),
            fleet: Arc::new(RwLock::new(cached_fleet)),
            fbos: Arc::new(RwLock::new(cached_fbos)),
            store: Arc::new(Mutex::new(store)),
            base_url,
        }
    }

    pub fn with_default_store(store: Store) -> Self {
        Self::with_store(DEFAULT_BASE_URL, store)
    }

    pub async fn connect(
        &self,
        company_id: String,
        api_key: String,
    ) -> Result<ConnectionStatus, ConnectionError> {
        let company_id =
            Uuid::parse_str(company_id.trim()).map_err(|_| ConnectionError::InvalidCompanyId)?;
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(ConnectionError::EmptyApiKey);
        }

        let client = Arc::new(
            OnAirClient::new(
                self.base_url,
                company_id,
                SecretString::from(api_key.to_owned()),
            )
            .map_err(classify_client_error)?,
        );
        let company = client
            .company_summary()
            .await
            .map_err(classify_client_error)?;

        let (cached_fleet, cached_fbos) = self.store.lock().ok().map_or((None, None), |store| {
            let storage = if store.is_persistent() {
                SnapshotStorage::Hoard
            } else {
                SnapshotStorage::MemoryOnly
            };
            (
                load_stored_fleet(&store, Some(&company.id))
                    .map(|stored| fleet_view(stored, SnapshotAvailability::Cached, storage)),
                load_stored_fbos(&store, Some(&company.id))
                    .map(|stored| fbo_view(stored, SnapshotAvailability::Cached, storage)),
            )
        });

        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(ConnectedSession {
            client,
            company,
            data_sync_gate: Arc::new(Mutex::new(DataSyncGate::default())),
        });
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_fleet;
        *self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_fbos;

        self.status()
    }

    pub fn disconnect(&self) -> Result<ConnectionStatus, ConnectionError> {
        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = None;
        if let Some(fleet) = self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            fleet.availability = SnapshotAvailability::Offline;
        }
        if let Some(fbos) = self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            fbos.availability = SnapshotAvailability::Offline;
        }
        self.status()
    }

    pub fn status(&self) -> Result<ConnectionStatus, ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        Ok(ConnectionStatus {
            connected: session.is_some(),
            company: session
                .as_ref()
                .map(|connected| ConnectedCompany::from(&connected.company)),
            credential_storage: "session_only",
        })
    }

    pub async fn synchronize_company_data(
        &self,
        trigger: DataSyncTrigger,
    ) -> Result<CompanyDataSyncResult, ConnectionError> {
        let (company, client, data_sync_gate) = {
            let session = self
                .inner
                .read()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
            (
                connected.company.clone(),
                Arc::clone(&connected.client),
                Arc::clone(&connected.data_sync_gate),
            )
        };

        let _sync_permit = {
            let mut gate = data_sync_gate
                .lock()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            if !gate.try_start(trigger, Instant::now()) {
                return Ok(CompanyDataSyncResult {
                    disposition: DataSyncDisposition::QuietlyIgnored,
                    fleet: self.fleet_snapshot()?,
                    fbos: self.fbo_snapshot()?,
                    failures: Vec::new(),
                });
            }
            DataSyncPermit {
                gate: Arc::clone(&data_sync_gate),
            }
        };

        let mut failures = Vec::new();
        let mut stop_after_fleet = false;
        let fleet = match client.fleet().await {
            Ok(snapshot) => Some(self.accept_fleet_snapshot(&company, snapshot)?),
            Err(error) => {
                stop_after_fleet = matches!(
                    error,
                    ClientError::AuthenticationRejected | ClientError::RateLimited
                );
                self.mark_fleet_cached(&company.id)?;
                failures.push(DataSyncFailure {
                    resource: CompanyDataResource::Fleet,
                    message: classify_resource_error(error, CompanyDataResource::Fleet).to_string(),
                });
                self.fleet_snapshot()?
            }
        };

        let fbos = if stop_after_fleet {
            self.mark_fbos_cached(&company.id)?;
            failures.push(DataSyncFailure {
                resource: CompanyDataResource::Fbos,
                message: "FBO synchronization was skipped to avoid another rejected request."
                    .to_owned(),
            });
            self.fbo_snapshot()?
        } else {
            match client.fbos().await {
                Ok(snapshot) => Some(self.accept_fbo_snapshot(&company, snapshot)?),
                Err(error) => {
                    self.mark_fbos_cached(&company.id)?;
                    failures.push(DataSyncFailure {
                        resource: CompanyDataResource::Fbos,
                        message: classify_resource_error(error, CompanyDataResource::Fbos)
                            .to_string(),
                    });
                    self.fbo_snapshot()?
                }
            }
        };

        Ok(CompanyDataSyncResult {
            disposition: DataSyncDisposition::Synchronized,
            fleet,
            fbos,
            failures,
        })
    }

    fn accept_fleet_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<Vec<AircraftSummary>>,
    ) -> Result<FleetSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        let stored = StoredFleetSnapshot {
            schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_fleet(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = fleet_view(stored, SnapshotAvailability::Live, storage);
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(view)
    }

    fn accept_fbo_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<Vec<FboSummary>>,
    ) -> Result<FboSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        let stored = StoredFboSnapshot {
            schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_fbos(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = fbo_view(stored, SnapshotAvailability::Live, storage);
        *self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(view)
    }

    pub fn fleet_snapshot(&self) -> Result<Option<FleetSnapshotView>, ConnectionError> {
        self.fleet
            .read()
            .map(|fleet| fleet.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    pub fn fbo_snapshot(&self) -> Result<Option<FboSnapshotView>, ConnectionError> {
        self.fbos
            .read()
            .map(|fbos| fbos.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    fn ensure_current_company(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
        (&connected.company.id == company_id)
            .then_some(())
            .ok_or(ConnectionError::StateUnavailable)
    }

    fn mark_fleet_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(fleet) = self
                .fleet
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            fleet.availability = SnapshotAvailability::Cached;
        }
        Ok(())
    }

    fn mark_fbos_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(fbos) = self
                .fbos
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            fbos.availability = SnapshotAvailability::Cached;
        }
        Ok(())
    }
}

fn fleet_view(
    stored: StoredFleetSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> FleetSnapshotView {
    FleetSnapshotView {
        company: ConnectedCompany::from(&stored.company),
        snapshot: stored.snapshot,
        availability,
        storage,
    }
}

fn fbo_view(
    stored: StoredFboSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> FboSnapshotView {
    FboSnapshotView {
        company: ConnectedCompany::from(&stored.company),
        snapshot: stored.snapshot,
        availability,
        storage,
    }
}

fn load_stored_fleet(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredFleetSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(FLEET_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    let stored: StoredFleetSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == FLEET_SNAPSHOT_SCHEMA_VERSION
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_fleet(store: &mut Store, stored: &StoredFleetSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            FLEET_RESOURCE_KIND,
            &stored.company.id.0.to_string(),
            &stored.snapshot.provenance.observed_at.to_rfc3339(),
            &payload,
        )
        .map_err(|_| ())
}

fn load_stored_fbos(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredFboSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(FBOS_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    let stored: StoredFboSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == FBOS_SNAPSHOT_SCHEMA_VERSION
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_fbos(store: &mut Store, stored: &StoredFboSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            FBOS_RESOURCE_KIND,
            &stored.company.id.0.to_string(),
            &stored.snapshot.provenance.observed_at.to_rfc3339(),
            &payload,
        )
        .map_err(|_| ())
}

fn classify_client_error(error: ClientError) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected | ClientError::ApiRejected => {
            ConnectionError::AuthenticationRejected
        }
        ClientError::CompanyNotFound => ConnectionError::CompanyNotFound,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => ConnectionError::ServiceUnavailable,
    }
}

fn classify_resource_error(error: ClientError, resource: CompanyDataResource) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected => ConnectionError::AuthenticationRejected,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => match resource {
            CompanyDataResource::Fleet => ConnectionError::FleetUnavailable,
            CompanyDataResource::Fbos => ConnectionError::FbosUnavailable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;
    use wyrmgrid_domain::{
        AircraftId, AirportId, AirportSummary, FboId, Provenance, ProvenanceKind,
    };

    #[derive(Default)]
    struct MemoryLegalPreferences {
        value: Mutex<Option<PersistedLegalPreferences>>,
    }

    impl LegalPreferencesRepository for MemoryLegalPreferences {
        fn load_legal_preferences(
            &self,
        ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
            self.value
                .lock()
                .map(|value| value.clone())
                .map_err(|_| LegalSettingsError::StorageUnavailable)
        }

        fn save_legal_preferences(
            &self,
            terms_version: &str,
            privacy_notice_version: &str,
            telemetry_enabled: bool,
        ) -> Result<(), LegalSettingsError> {
            *self
                .value
                .lock()
                .map_err(|_| LegalSettingsError::StorageUnavailable)? =
                Some(PersistedLegalPreferences {
                    terms_version: terms_version.to_owned(),
                    privacy_notice_version: privacy_notice_version.to_owned(),
                    telemetry_enabled,
                    acknowledged_at: "2026-07-14 00:00:00".to_owned(),
                });
            Ok(())
        }
    }

    #[test]
    fn exposes_the_supported_plugin_api() {
        assert_eq!(platform_status().plugin_api_version, 1);
    }

    #[test]
    fn legal_documents_require_versioned_acknowledgement() {
        let service = LegalSettingsService::new(MemoryLegalPreferences::default());
        assert_eq!(
            service.status().expect("status should be available"),
            LegalStatus {
                terms_version: TERMS_VERSION,
                privacy_notice_version: PRIVACY_NOTICE_VERSION,
                acknowledged: false,
                telemetry_enabled: false,
                acknowledged_at: None,
            }
        );

        let accepted = service
            .acknowledge(true)
            .expect("preferences should be saved");
        assert!(accepted.acknowledged);
        assert!(accepted.telemetry_enabled);
        assert_eq!(
            accepted.acknowledged_at.as_deref(),
            Some("2026-07-14 00:00:00")
        );

        let updated = service
            .update_telemetry(false)
            .expect("telemetry preference should be saved");
        assert!(!updated.telemetry_enabled);
    }

    #[test]
    fn old_legal_versions_disable_telemetry_until_reviewed() {
        let repository = MemoryLegalPreferences::default();
        repository
            .save_legal_preferences("2026-01-01", "2026-01-01", true)
            .expect("fixture should be saved");
        let service = LegalSettingsService::new(repository);

        let status = service.status().expect("status should be available");
        assert!(!status.acknowledged);
        assert!(!status.telemetry_enabled);
        assert!(matches!(
            service.update_telemetry(true),
            Err(LegalSettingsError::AcknowledgementRequired)
        ));
    }

    #[test]
    fn starts_disconnected_without_persistent_credentials() {
        let session = OnAirSession::default();
        assert_eq!(
            session.status().expect("status should be available"),
            ConnectionStatus {
                connected: false,
                company: None,
                credential_storage: "session_only",
            }
        );
    }

    #[test]
    fn restores_the_latest_persistent_company_data_as_offline() {
        let directory = tempdir().expect("temporary Hoard directory should exist");
        let database_path = directory.path().join("wyrmgrid.db");
        let company = CompanySummary {
            id: CompanyId(Uuid::new_v4()),
            name: "Cached Charter".into(),
            airline_code: "CCH".into(),
        };
        let stored = StoredFleetSnapshot {
            schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot: Observed {
                value: vec![AircraftSummary {
                    id: AircraftId(Uuid::new_v4()),
                    registration: Some("CACHE-1".into()),
                    model: Some("Stored Aircraft".into()),
                    location: None,
                    current_airport: None,
                }],
                provenance: Provenance {
                    kind: ProvenanceKind::OnAirFact,
                    source: "onair:company/fleet".into(),
                    observed_at: Utc::now(),
                },
            },
        };
        let stored_fbos = StoredFboSnapshot {
            schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot: Observed {
                value: vec![FboSummary {
                    id: FboId(Uuid::new_v4()),
                    name: Some("Cached Aerie".into()),
                    airport: Some(AirportSummary {
                        id: AirportId(Uuid::new_v4()),
                        icao: Some("YTEST".into()),
                        name: Some("Stored Airport".into()),
                        location: None,
                    }),
                }],
                provenance: Provenance {
                    kind: ProvenanceKind::OnAirFact,
                    source: "onair:company/fbos".into(),
                    observed_at: Utc::now(),
                },
            },
        };
        let mut store = Store::open(&database_path).expect("persistent Hoard should open");
        save_stored_fleet(&mut store, &stored).expect("fleet should persist");
        save_stored_fbos(&mut store, &stored_fbos).expect("FBOs should persist");
        drop(store);

        let session = OnAirSession::with_store(
            DEFAULT_BASE_URL,
            Store::open(&database_path).expect("persistent Hoard should reopen"),
        );
        let fleet_view = session
            .fleet_snapshot()
            .expect("fleet state should be readable")
            .expect("cached fleet should restore");
        let fbo_view = session
            .fbo_snapshot()
            .expect("FBO state should be readable")
            .expect("cached FBOs should restore");

        assert_eq!(fleet_view.company, ConnectedCompany::from(&company));
        assert_eq!(fleet_view.availability, SnapshotAvailability::Offline);
        assert_eq!(fleet_view.storage, SnapshotStorage::Hoard);
        assert_eq!(fleet_view.snapshot, stored.snapshot);
        assert_eq!(fbo_view.company, ConnectedCompany::from(&company));
        assert_eq!(fbo_view.availability, SnapshotAvailability::Offline);
        assert_eq!(fbo_view.storage, SnapshotStorage::Hoard);
        assert_eq!(fbo_view.snapshot, stored_fbos.snapshot);
    }

    #[tokio::test]
    async fn rejects_invalid_credentials_before_network_access() {
        let session = OnAirSession::default();
        assert!(matches!(
            session.connect("not-a-uuid".into(), "secret".into()).await,
            Err(ConnectionError::InvalidCompanyId)
        ));
        assert!(matches!(
            session.connect(Uuid::nil().to_string(), "  ".into()).await,
            Err(ConnectionError::EmptyApiKey)
        ));
    }

    #[tokio::test]
    async fn refuses_company_sync_without_a_connected_session() {
        let session = OnAirSession::default();
        assert!(matches!(
            session
                .synchronize_company_data(DataSyncTrigger::Manual)
                .await,
            Err(ConnectionError::NotConnected)
        ));
        assert_eq!(
            session
                .fleet_snapshot()
                .expect("snapshot state should be readable"),
            None
        );
        assert_eq!(
            session
                .fbo_snapshot()
                .expect("snapshot state should be readable"),
            None
        );
    }

    #[test]
    fn data_sync_gate_enforces_trigger_specific_quiet_periods() {
        let started = Instant::now();
        let mut gate = DataSyncGate::default();

        assert!(gate.try_start(DataSyncTrigger::Initial, started));
        assert!(!gate.try_start(DataSyncTrigger::Manual, started));
        gate.finish();
        assert!(!gate.try_start(
            DataSyncTrigger::Manual,
            started + MANUAL_SYNC_COOLDOWN - Duration::from_secs(1)
        ));
        assert!(gate.try_start(DataSyncTrigger::Manual, started + MANUAL_SYNC_COOLDOWN));
        gate.finish();
        assert!(!gate.try_start(
            DataSyncTrigger::Automatic,
            started + MANUAL_SYNC_COOLDOWN + Duration::from_secs(1)
        ));
        assert!(gate.try_start(
            DataSyncTrigger::Automatic,
            started + MANUAL_SYNC_COOLDOWN + MINIMUM_AUTOMATIC_SYNC_INTERVAL
        ));
    }

    #[test]
    fn maps_adapter_failures_to_bounded_user_messages() {
        assert!(matches!(
            classify_client_error(ClientError::AuthenticationRejected),
            ConnectionError::AuthenticationRejected
        ));
        assert!(matches!(
            classify_client_error(ClientError::RateLimited),
            ConnectionError::RateLimited
        ));
        assert!(matches!(
            classify_client_error(ClientError::CompanyNotFound),
            ConnectionError::CompanyNotFound
        ));
        let message = ConnectionError::AuthenticationRejected.to_string();
        assert!(message.contains("For now"));
        assert!(message.contains("not OnAir Companion"));
        assert!(matches!(
            classify_client_error(ClientError::MissingContent),
            ConnectionError::ServiceUnavailable
        ));
        assert!(matches!(
            classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fleet),
            ConnectionError::FleetUnavailable
        ));
        assert!(matches!(
            classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fbos),
            ConnectionError::FbosUnavailable
        ));
    }

    #[test]
    fn exposes_stable_safe_operation_errors() {
        assert_eq!(
            OperationError::from(ConnectionError::RateLimited),
            OperationError {
                code: "onair.rate_limited",
                message: ConnectionError::RateLimited.to_string(),
                retryable: true,
                reportable: false,
                report_id: None,
            }
        );
        assert!(OperationError::from(ConnectionError::StateUnavailable).reportable);
        assert!(!OperationError::from(ConnectionError::AuthenticationRejected).reportable);
    }
}

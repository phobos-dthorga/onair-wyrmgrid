//! Application-level orchestration independent of Tauri and other interfaces.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{AircraftSummary, CompanyId, CompanySummary, Observed};
use wyrmgrid_onair_api::{ClientError, DEFAULT_BASE_URL, OnAirClient};
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;
use wyrmgrid_storage::Store;

const FLEET_RESOURCE_KIND: &str = "onair_company_fleet";
const FLEET_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

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

pub const MANUAL_FLEET_SYNC_COOLDOWN: Duration = Duration::from_secs(60);
pub const MINIMUM_AUTOMATIC_FLEET_SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSyncTrigger {
    Initial,
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSyncDisposition {
    Synchronized,
    QuietlyIgnored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSnapshotAvailability {
    Live,
    Cached,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSnapshotStorage {
    Hoard,
    MemoryOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<Vec<AircraftSummary>>,
    pub availability: FleetSnapshotAvailability,
    pub storage: FleetSnapshotStorage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSyncResult {
    pub disposition: FleetSyncDisposition,
    pub snapshot: Option<FleetSnapshotView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredFleetSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<Vec<AircraftSummary>>,
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
    #[error("Connect to OnAir before refreshing the fleet.")]
    NotConnected,
    #[error(
        "WyrmGrid could not refresh the fleet. A previous successful observation, if present, remains available."
    )]
    FleetUnavailable,
}

#[derive(Clone)]
pub struct OnAirSession {
    inner: Arc<RwLock<Option<ConnectedSession>>>,
    fleet: Arc<RwLock<Option<FleetSnapshotView>>>,
    store: Arc<Mutex<Store>>,
    base_url: &'static str,
}

struct ConnectedSession {
    client: Arc<OnAirClient>,
    company: CompanySummary,
    fleet_sync_gate: Arc<Mutex<FleetSyncGate>>,
}

#[derive(Debug, Default)]
struct FleetSyncGate {
    in_progress: bool,
    last_started: Option<Instant>,
}

impl FleetSyncGate {
    fn try_start(&mut self, trigger: FleetSyncTrigger, now: Instant) -> bool {
        if self.in_progress {
            return false;
        }

        let minimum_interval = match trigger {
            FleetSyncTrigger::Initial => Duration::ZERO,
            FleetSyncTrigger::Manual => MANUAL_FLEET_SYNC_COOLDOWN,
            FleetSyncTrigger::Automatic => MINIMUM_AUTOMATIC_FLEET_SYNC_INTERVAL,
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

struct FleetSyncPermit {
    gate: Arc<Mutex<FleetSyncGate>>,
}

impl Drop for FleetSyncPermit {
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
        let cached = load_stored_fleet(&store, None).map(|stored| {
            fleet_view(
                stored,
                FleetSnapshotAvailability::Offline,
                if persistent {
                    FleetSnapshotStorage::Hoard
                } else {
                    FleetSnapshotStorage::MemoryOnly
                },
            )
        });
        Self {
            inner: Arc::new(RwLock::new(None)),
            fleet: Arc::new(RwLock::new(cached)),
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

        let cached = self
            .store
            .lock()
            .ok()
            .and_then(|store| load_stored_fleet(&store, Some(&company.id)))
            .map(|stored| {
                fleet_view(
                    stored,
                    FleetSnapshotAvailability::Cached,
                    FleetSnapshotStorage::Hoard,
                )
            });

        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(ConnectedSession {
            client,
            company,
            fleet_sync_gate: Arc::new(Mutex::new(FleetSyncGate::default())),
        });
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached;

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
            fleet.availability = FleetSnapshotAvailability::Offline;
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

    pub async fn synchronize_fleet(
        &self,
        trigger: FleetSyncTrigger,
    ) -> Result<FleetSyncResult, ConnectionError> {
        let (company_id, client, fleet_sync_gate) = {
            let session = self
                .inner
                .read()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
            (
                connected.company.id.clone(),
                Arc::clone(&connected.client),
                Arc::clone(&connected.fleet_sync_gate),
            )
        };

        let _sync_permit = {
            let mut gate = fleet_sync_gate
                .lock()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            if !gate.try_start(trigger, Instant::now()) {
                return Ok(FleetSyncResult {
                    disposition: FleetSyncDisposition::QuietlyIgnored,
                    snapshot: self.fleet_snapshot()?,
                });
            }
            FleetSyncPermit {
                gate: Arc::clone(&fleet_sync_gate),
            }
        };

        let fleet = match client.fleet().await {
            Ok(fleet) => fleet,
            Err(error) => {
                self.mark_fleet_cached(&company_id)?;
                return Err(classify_fleet_error(error));
            }
        };

        let company = {
            let session = self
                .inner
                .read()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
            if connected.company.id != company_id {
                return Err(ConnectionError::StateUnavailable);
            }
            connected.company.clone()
        };
        let stored = StoredFleetSnapshot {
            schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
            company,
            snapshot: fleet,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_fleet(&mut store, &stored).ok())
            .map_or(FleetSnapshotStorage::MemoryOnly, |_| {
                FleetSnapshotStorage::Hoard
            });
        let view = fleet_view(stored, FleetSnapshotAvailability::Live, storage);
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(FleetSyncResult {
            disposition: FleetSyncDisposition::Synchronized,
            snapshot: Some(view),
        })
    }

    pub fn fleet_snapshot(&self) -> Result<Option<FleetSnapshotView>, ConnectionError> {
        self.fleet
            .read()
            .map(|fleet| fleet.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
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
            fleet.availability = FleetSnapshotAvailability::Cached;
        }
        Ok(())
    }
}

fn fleet_view(
    stored: StoredFleetSnapshot,
    availability: FleetSnapshotAvailability,
    storage: FleetSnapshotStorage,
) -> FleetSnapshotView {
    FleetSnapshotView {
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

fn classify_fleet_error(error: ClientError) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected => ConnectionError::AuthenticationRejected,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => ConnectionError::FleetUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;
    use wyrmgrid_domain::{AircraftId, Provenance, ProvenanceKind};

    #[test]
    fn exposes_the_supported_plugin_api() {
        assert_eq!(platform_status().plugin_api_version, 1);
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
    fn restores_the_latest_persistent_fleet_as_offline_data() {
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
        let mut store = Store::open(&database_path).expect("persistent Hoard should open");
        save_stored_fleet(&mut store, &stored).expect("fleet should persist");
        drop(store);

        let session = OnAirSession::with_store(
            DEFAULT_BASE_URL,
            Store::open(&database_path).expect("persistent Hoard should reopen"),
        );
        let view = session
            .fleet_snapshot()
            .expect("fleet state should be readable")
            .expect("cached fleet should restore");

        assert_eq!(view.company, ConnectedCompany::from(&company));
        assert_eq!(view.availability, FleetSnapshotAvailability::Offline);
        assert_eq!(view.storage, FleetSnapshotStorage::Hoard);
        assert_eq!(view.snapshot, stored.snapshot);
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
    async fn refuses_fleet_refresh_without_a_connected_session() {
        let session = OnAirSession::default();
        assert!(matches!(
            session.synchronize_fleet(FleetSyncTrigger::Manual).await,
            Err(ConnectionError::NotConnected)
        ));
        assert_eq!(
            session
                .fleet_snapshot()
                .expect("snapshot state should be readable"),
            None
        );
    }

    #[test]
    fn fleet_sync_gate_enforces_trigger_specific_quiet_periods() {
        let started = Instant::now();
        let mut gate = FleetSyncGate::default();

        assert!(gate.try_start(FleetSyncTrigger::Initial, started));
        assert!(!gate.try_start(FleetSyncTrigger::Manual, started));
        gate.finish();
        assert!(!gate.try_start(
            FleetSyncTrigger::Manual,
            started + MANUAL_FLEET_SYNC_COOLDOWN - Duration::from_secs(1)
        ));
        assert!(gate.try_start(
            FleetSyncTrigger::Manual,
            started + MANUAL_FLEET_SYNC_COOLDOWN
        ));
        gate.finish();
        assert!(!gate.try_start(
            FleetSyncTrigger::Automatic,
            started + MANUAL_FLEET_SYNC_COOLDOWN + Duration::from_secs(1)
        ));
        assert!(gate.try_start(
            FleetSyncTrigger::Automatic,
            started + MANUAL_FLEET_SYNC_COOLDOWN + MINIMUM_AUTOMATIC_FLEET_SYNC_INTERVAL
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
            classify_fleet_error(ClientError::ApiRejected),
            ConnectionError::FleetUnavailable
        ));
    }
}

//! Application-level orchestration independent of Tauri and other interfaces.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{AircraftSummary, CompanySummary, Observed};
use wyrmgrid_onair_api::{ClientError, DEFAULT_BASE_URL, OnAirClient};
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;

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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSyncResult {
    pub disposition: FleetSyncDisposition,
    pub snapshot: Option<Observed<Vec<AircraftSummary>>>,
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
    base_url: &'static str,
}

struct ConnectedSession {
    client: Arc<OnAirClient>,
    company: CompanySummary,
    fleet: Option<Observed<Vec<AircraftSummary>>>,
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
        Self {
            inner: Arc::new(RwLock::new(None)),
            base_url,
        }
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

        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(ConnectedSession {
            client,
            company,
            fleet: None,
            fleet_sync_gate: Arc::new(Mutex::new(FleetSyncGate::default())),
        });

        self.status()
    }

    pub fn disconnect(&self) -> Result<ConnectionStatus, ConnectionError> {
        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = None;
        self.status()
    }

    pub fn status(&self) -> Result<ConnectionStatus, ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        Ok(ConnectionStatus {
            connected: session.is_some(),
            company: session.as_ref().map(|connected| ConnectedCompany {
                name: connected.company.name.clone(),
                airline_code: connected.company.airline_code.clone(),
            }),
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

        let fleet = client.fleet().await.map_err(classify_fleet_error)?;

        let mut session = self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        let connected = session.as_mut().ok_or(ConnectionError::NotConnected)?;
        if connected.company.id != company_id {
            return Err(ConnectionError::StateUnavailable);
        }
        connected.fleet = Some(fleet.clone());
        Ok(FleetSyncResult {
            disposition: FleetSyncDisposition::Synchronized,
            snapshot: Some(fleet),
        })
    }

    pub fn fleet_snapshot(
        &self,
    ) -> Result<Option<Observed<Vec<AircraftSummary>>>, ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        Ok(session
            .as_ref()
            .and_then(|connected| connected.fleet.clone()))
    }
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

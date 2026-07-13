//! Application-level orchestration independent of Tauri and other interfaces.

use secrecy::SecretString;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::CompanySummary;
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
}

#[derive(Clone)]
pub struct OnAirSession {
    inner: Arc<RwLock<Option<ConnectedSession>>>,
    base_url: &'static str,
}

struct ConnectedSession {
    _client: OnAirClient,
    company: CompanySummary,
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

        let client = OnAirClient::new(
            self.base_url,
            company_id,
            SecretString::from(api_key.to_owned()),
        )
        .map_err(classify_client_error)?;
        let company = client
            .company_summary()
            .await
            .map_err(classify_client_error)?;

        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(ConnectedSession {
            _client: client,
            company,
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
}

fn classify_client_error(error: ClientError) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected
        | ClientError::ApiRejected
        | ClientError::MissingContent => ConnectionError::AuthenticationRejected,
        ClientError::CompanyNotFound => ConnectionError::CompanyNotFound,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => ConnectionError::ServiceUnavailable,
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
    }
}

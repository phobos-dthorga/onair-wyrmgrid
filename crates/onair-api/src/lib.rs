//! Read-only OnAir API boundary.
//!
//! Raw responses stay inside this crate. Other parts of WyrmGrid consume stable
//! domain models rather than binding themselves to OnAir's JSON field names.

use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use wyrmgrid_domain::{CompanyId, CompanySummary};

const API_KEY_HEADER: &str = "oa-apikey";
pub const DEFAULT_BASE_URL: &str = "https://server1.onair.company/api/v1";

pub struct OnAirClient {
    http: Client,
    base_url: Url,
    api_key: SecretString,
}

impl std::fmt::Debug for OnAirClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OnAirClient")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid OnAir API base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("invalid API key header")]
    InvalidApiKeyHeader(#[from] header::InvalidHeaderValue),
    #[error("OnAir request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("OnAir returned HTTP {0}")]
    Http(StatusCode),
    #[error("OnAir rejected the API key or company ID")]
    AuthenticationRejected,
    #[error("OnAir company was not found")]
    CompanyNotFound,
    #[error("OnAir rate limit reached")]
    RateLimited,
    #[error("OnAir rejected the request")]
    ApiRejected,
    #[error("OnAir returned an incomplete company response")]
    MissingContent,
}

#[derive(Debug, Deserialize)]
struct ApiResult<T> {
    #[serde(rename = "Content")]
    content: Option<T>,
    #[serde(rename = "Error", default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCompany {
    #[serde(rename = "Id")]
    id: Uuid,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AirlineCode", default)]
    airline_code: String,
}

impl OnAirClient {
    pub fn new(base_url: &str, api_key: SecretString) -> Result<Self, ClientError> {
        let mut parsed = Url::parse(base_url)?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path()));
        }

        Ok(Self {
            http: Client::new(),
            base_url: parsed,
            api_key,
        })
    }

    pub async fn company_summary(&self, company_id: Uuid) -> Result<CompanySummary, ClientError> {
        let response: ApiResult<RawCompany> = self.get(&format!("company/{company_id}")).await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let company = response.content.ok_or(ClientError::MissingContent)?;
        Ok(CompanySummary {
            id: CompanyId(company.id),
            name: company.name,
            airline_code: company.airline_code,
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = self.base_url.join(path)?;
        let api_key = header::HeaderValue::from_str(self.api_key.expose_secret())?;
        let response = self
            .http
            .get(url)
            .header(API_KEY_HEADER, api_key)
            .header(header::ACCEPT, "application/json")
            .send()
            .await?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(ClientError::AuthenticationRejected);
            }
            StatusCode::NOT_FOUND => return Err(ClientError::CompanyNotFound),
            StatusCode::TOO_MANY_REQUESTS => return Err(ClientError::RateLimited),
            status if !status.is_success() => return Err(ClientError::Http(status)),
            _ => {}
        }

        Ok(response.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_api_key() {
        let client = OnAirClient::new(
            "https://server1.onair.company/api/v1",
            SecretString::from("super-secret-key".to_owned()),
        )
        .expect("client should be valid");

        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret-key"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn translates_the_swagger_company_envelope() {
        let response: ApiResult<RawCompany> = serde_json::from_str(include_str!(
            "../tests/fixtures/swagger-company-response.json"
        ))
        .expect("synthetic Swagger fixture should deserialize");
        let company = response.content.expect("fixture should contain a company");

        assert_eq!(company.name, "Example Air");
        assert_eq!(company.airline_code, "WYR");
    }

    #[test]
    fn never_exposes_the_remote_error_body() {
        let response: ApiResult<RawCompany> =
            serde_json::from_str(r#"{"Content":null,"Error":"credential-specific remote detail"}"#)
                .expect("error envelope should deserialize");

        assert!(response.error.is_some());
        assert_eq!(
            ClientError::ApiRejected.to_string(),
            "OnAir rejected the request"
        );
        assert!(
            !ClientError::ApiRejected
                .to_string()
                .contains("credential-specific")
        );
    }
}

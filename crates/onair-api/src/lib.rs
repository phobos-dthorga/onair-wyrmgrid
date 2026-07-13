//! Read-only OnAir API boundary.
//!
//! Raw responses stay inside this crate. Other parts of WyrmGrid consume stable
//! domain models rather than binding themselves to OnAir's JSON field names.

use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const API_KEY_HEADER: &str = "oa-apikey";

#[derive(Clone)]
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

    pub async fn company<T: DeserializeOwned>(&self, company_id: Uuid) -> Result<T, ClientError> {
        self.get(&format!("company/{company_id}")).await
    }

    pub async fn fleet<T: DeserializeOwned>(&self, company_id: Uuid) -> Result<T, ClientError> {
        self.get(&format!("company/{company_id}/fleet")).await
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

        if !response.status().is_success() {
            return Err(ClientError::Http(response.status()));
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
}

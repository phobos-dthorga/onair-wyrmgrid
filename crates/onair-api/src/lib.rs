//! Read-only OnAir API boundary.
//!
//! Raw responses stay inside this crate. Other parts of WyrmGrid consume stable
//! domain models rather than binding themselves to OnAir's JSON field names.

use chrono::Utc;
use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftId, AircraftSummary, AirportId, AirportSummary, CompanyId, CompanySummary, Coordinates,
    Observed, Provenance, ProvenanceKind,
};

const API_KEY_HEADER: &str = "oa-apikey";
const COMPANY_ID_HEADER: &str = "CompanyUniqueId";
const FLEET_PROVENANCE_SOURCE: &str = "onair:company/fleet";
pub const DEFAULT_BASE_URL: &str = "https://server1.onair.company/api/v1";

pub struct OnAirClient {
    http: Client,
    base_url: Url,
    company_id: Uuid,
    api_key: SecretString,
}

impl std::fmt::Debug for OnAirClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OnAirClient")
            .field("base_url", &self.base_url)
            .field("company_id", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid OnAir API base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("invalid OnAir request header")]
    InvalidRequestHeader(#[from] header::InvalidHeaderValue),
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

#[derive(Debug, Deserialize)]
struct RawAircraft {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "Identifier", default)]
    identifier: Option<String>,
    #[serde(rename = "AircraftType", default)]
    aircraft_type: Option<RawAircraftType>,
    #[serde(rename = "Longitude", default)]
    longitude: Option<f64>,
    #[serde(rename = "Latitude", default)]
    latitude: Option<f64>,
    #[serde(rename = "CurrentAirport", default)]
    current_airport: Option<RawAirport>,
}

#[derive(Debug, Deserialize)]
struct RawAircraftType {
    #[serde(rename = "DisplayName", default)]
    display_name: Option<String>,
    #[serde(rename = "TypeName", default)]
    type_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAirport {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "ICAO", default)]
    icao: Option<String>,
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "Longitude", default)]
    longitude: Option<f64>,
    #[serde(rename = "Latitude", default)]
    latitude: Option<f64>,
}

impl OnAirClient {
    pub fn new(
        base_url: &str,
        company_id: Uuid,
        api_key: SecretString,
    ) -> Result<Self, ClientError> {
        let mut parsed = Url::parse(base_url)?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path()));
        }

        Ok(Self {
            http: Client::new(),
            base_url: parsed,
            company_id,
            api_key,
        })
    }

    pub async fn company_summary(&self) -> Result<CompanySummary, ClientError> {
        let response: ApiResult<RawCompany> =
            self.get(&format!("company/{}", self.company_id)).await?;
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

    pub async fn fleet(&self) -> Result<Observed<Vec<AircraftSummary>>, ClientError> {
        let response: ApiResult<Vec<RawAircraft>> = self
            .get(&format!("company/{}/fleet", self.company_id))
            .await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let aircraft = response
            .content
            .ok_or(ClientError::MissingContent)?
            .into_iter()
            .filter_map(translate_aircraft)
            .collect();

        Ok(Observed {
            value: aircraft,
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: FLEET_PROVENANCE_SOURCE.to_owned(),
                observed_at: Utc::now(),
            },
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let response = self.http.execute(self.request(path)?).await?;

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

    fn request(&self, path: &str) -> Result<reqwest::Request, ClientError> {
        let url = self.base_url.join(path)?;
        let api_key = header::HeaderValue::from_str(self.api_key.expose_secret())?;
        let company_id = header::HeaderValue::from_str(&self.company_id.to_string())?;

        Ok(self
            .http
            .get(url)
            .header(API_KEY_HEADER, api_key)
            .header(COMPANY_ID_HEADER, company_id)
            .header(header::ACCEPT, "application/json")
            .build()?)
    }
}

fn translate_aircraft(aircraft: RawAircraft) -> Option<AircraftSummary> {
    let current_airport = aircraft.current_airport.and_then(translate_airport);
    let direct_location = coordinates(aircraft.latitude, aircraft.longitude);
    let airport_location = current_airport
        .as_ref()
        .and_then(|airport| airport.location);

    Some(AircraftSummary {
        id: AircraftId(aircraft.id?),
        registration: non_empty(aircraft.identifier),
        model: aircraft.aircraft_type.and_then(|aircraft_type| {
            non_empty(aircraft_type.display_name).or_else(|| non_empty(aircraft_type.type_name))
        }),
        location: direct_location.or(airport_location),
        current_airport,
    })
}

fn translate_airport(airport: RawAirport) -> Option<AirportSummary> {
    Some(AirportSummary {
        id: AirportId(airport.id?),
        icao: non_empty(airport.icao),
        name: non_empty(airport.name),
        location: coordinates(airport.latitude, airport.longitude),
    })
}

fn coordinates(latitude: Option<f64>, longitude: Option<f64>) -> Option<Coordinates> {
    let coordinates = Coordinates {
        latitude: latitude?,
        longitude: longitude?,
    };
    coordinates.is_valid().then_some(coordinates)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_api_key() {
        let client = OnAirClient::new(
            "https://server1.onair.company/api/v1",
            Uuid::nil(),
            SecretString::from("super-secret-key".to_owned()),
        )
        .expect("client should be valid");

        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret-key"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn company_requests_include_both_observed_authentication_headers() {
        let company_id = Uuid::parse_str("11111111-2222-4333-8444-555555555555")
            .expect("test company ID should be valid");
        let client = OnAirClient::new(
            "https://server1.onair.company/api/v1",
            company_id,
            SecretString::from("synthetic-test-key".to_owned()),
        )
        .expect("client should be valid");

        let request = client
            .request(&format!("company/{company_id}"))
            .expect("request should build");

        assert_eq!(
            request.headers().get(API_KEY_HEADER),
            Some(&header::HeaderValue::from_static("synthetic-test-key"))
        );
        assert_eq!(
            request.headers().get(COMPANY_ID_HEADER),
            Some(
                &header::HeaderValue::from_str(&company_id.to_string())
                    .expect("company ID should be a valid header")
            )
        );
        assert_eq!(
            request.url().as_str(),
            format!("https://server1.onair.company/api/v1/company/{company_id}")
        );
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
    fn translates_the_swagger_fleet_envelope_without_inventing_missing_facts() {
        let response: ApiResult<Vec<RawAircraft>> = serde_json::from_str(include_str!(
            "../tests/fixtures/swagger-fleet-response.json"
        ))
        .expect("synthetic Swagger fixture should deserialize");
        let aircraft: Vec<_> = response
            .content
            .expect("fixture should contain a fleet")
            .into_iter()
            .filter_map(translate_aircraft)
            .collect();

        assert_eq!(aircraft.len(), 3);
        assert_eq!(aircraft[0].registration.as_deref(), Some("WYR-101"));
        assert_eq!(aircraft[0].model.as_deref(), Some("Example Turboprop"));
        assert_eq!(
            aircraft[0]
                .current_airport
                .as_ref()
                .and_then(|airport| airport.icao.as_deref()),
            Some("YTEST")
        );
        assert_eq!(
            aircraft[1].location,
            Some(Coordinates {
                latitude: -33.86,
                longitude: 151.2,
            })
        );
        assert_eq!(aircraft[2].registration, None);
        assert_eq!(aircraft[2].model, None);
        assert_eq!(aircraft[2].location, None);
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

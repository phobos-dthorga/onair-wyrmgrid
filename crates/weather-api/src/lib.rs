//! Read-only, bounded AviationWeather.gov airport-weather adapter.
//!
//! Raw provider JSON remains private to this crate. Public callers receive only
//! validated WyrmGrid weather snapshots and stable, body-free error categories.

use chrono::{DateTime, Utc};
use reqwest::{StatusCode, header, redirect::Policy};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use wyrmgrid_domain::{
    AirportWeather, FlightCategory, MAX_WEATHER_AIRPORTS, MetarObservation, OperationalObservation,
    OperationalProvenance, ProvenanceKind, TafForecast, WEATHER_SNAPSHOT_SCHEMA_VERSION,
    WeatherSnapshot, WeatherSnapshotId, WindDirection, weather_freshness,
};

pub const DEFAULT_BASE_URL: &str = "https://aviationweather.gov/api/data/";
pub const MAX_RESPONSE_BYTES: usize = 512 * 1024;
pub const TRANSFORMATION_VERSION: u32 = 1;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const PROVIDER_REVISION: &str = "data-api-v4";

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    #[error("The aviation weather provider could not be initialized.")]
    ConfigurationUnavailable,
    #[error("The flight plan contains an invalid airport weather station identifier.")]
    InvalidStations,
    #[error("AviationWeather.gov is rate-limiting requests. Wait before trying again.")]
    RateLimited,
    #[error("The aviation weather request timed out.")]
    TimedOut,
    #[error("AviationWeather.gov is unreachable from this device right now.")]
    Offline,
    #[error("AviationWeather.gov is temporarily unavailable.")]
    ProviderUnavailable,
    #[error("AviationWeather.gov returned an unexpected response.")]
    UnexpectedResponse,
    #[error("The aviation weather response exceeded WyrmGrid's 512 KiB safety limit.")]
    ResponseTooLarge,
    #[error("AviationWeather.gov returned a response that was not JSON.")]
    InvalidContentType,
    #[error("The aviation weather response did not match WyrmGrid's validated import contract.")]
    MalformedWeather,
}

#[derive(Clone)]
pub struct AviationWeatherClient {
    http: reqwest::Client,
    base_url: Url,
}

impl AviationWeatherClient {
    pub fn new() -> Result<Self, ClientError> {
        let base_url =
            Url::parse(DEFAULT_BASE_URL).map_err(|_| ClientError::ConfigurationUnavailable)?;
        Self::with_base_url(base_url)
    }

    fn with_base_url(base_url: Url) -> Result<Self, ClientError> {
        if base_url.scheme() != "https" && !cfg!(test) {
            return Err(ClientError::ConfigurationUnavailable);
        }
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!(
                "OnAir-WyrmGrid/",
                env!("CARGO_PKG_VERSION"),
                " (flight-simulation planning)"
            ))
            .build()
            .map_err(|_| ClientError::ConfigurationUnavailable)?;
        Ok(Self { http, base_url })
    }

    pub async fn fetch_airports(
        &self,
        stations: &[String],
    ) -> Result<WeatherSnapshot, ClientError> {
        let stations = normalize_stations(stations)?;
        let station_query = stations.join(",");
        let metar = self.fetch_product("metar", &station_query).await?;
        let taf = self.fetch_product("taf", &station_query).await?;
        translate_airport_weather(&stations, &metar, &taf, Utc::now())
    }

    async fn fetch_product(&self, product: &str, stations: &str) -> Result<Vec<u8>, ClientError> {
        let mut url = self
            .base_url
            .join(product)
            .map_err(|_| ClientError::ConfigurationUnavailable)?;
        url.query_pairs_mut()
            .append_pair("ids", stations)
            .append_pair("format", "json");

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(classify_transport_error)?;
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(b"[]".to_vec());
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(ClientError::RateLimited);
        }
        if status.is_redirection() {
            return Err(ClientError::UnexpectedResponse);
        }
        if status.is_server_error() {
            return Err(ClientError::ProviderUnavailable);
        }
        if status != StatusCode::OK {
            return Err(ClientError::UnexpectedResponse);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::ResponseTooLarge);
        }
        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            let content_type = content_type
                .to_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !content_type.starts_with("application/json")
                && !content_type.starts_with("text/json")
            {
                return Err(ClientError::InvalidContentType);
            }
        }

        let mut response = response;
        let mut bytes = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(8 * 1024)
                .min(MAX_RESPONSE_BYTES as u64) as usize,
        );
        while let Some(chunk) = response.chunk().await.map_err(classify_transport_error)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(ClientError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

fn normalize_stations(stations: &[String]) -> Result<Vec<String>, ClientError> {
    let mut normalized = stations
        .iter()
        .map(|station| station.trim().to_ascii_uppercase())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty()
        || normalized.len() > MAX_WEATHER_AIRPORTS
        || normalized.iter().any(|station| {
            station.len() != 4
                || !station
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
    {
        return Err(ClientError::InvalidStations);
    }
    Ok(normalized)
}

fn classify_transport_error(error: reqwest::Error) -> ClientError {
    if error.is_timeout() {
        ClientError::TimedOut
    } else if error.is_connect() {
        ClientError::Offline
    } else {
        ClientError::ProviderUnavailable
    }
}

pub fn translate_airport_weather(
    requested_stations: &[String],
    metar_bytes: &[u8],
    taf_bytes: &[u8],
    retrieved_at: DateTime<Utc>,
) -> Result<WeatherSnapshot, ClientError> {
    if metar_bytes.len() > MAX_RESPONSE_BYTES || taf_bytes.len() > MAX_RESPONSE_BYTES {
        return Err(ClientError::ResponseTooLarge);
    }
    let stations = normalize_stations(requested_stations)?;
    let metars = translate_metars(metar_bytes, &stations, retrieved_at)?;
    let tafs = translate_tafs(taf_bytes, &stations, retrieved_at)?;
    let airports = stations
        .into_iter()
        .map(|station_icao| AirportWeather {
            metar: metars.get(&station_icao).cloned(),
            taf: tafs.get(&station_icao).cloned(),
            station_icao,
        })
        .collect();
    let snapshot = WeatherSnapshot {
        schema_version: WEATHER_SNAPSHOT_SCHEMA_VERSION,
        id: WeatherSnapshotId(Uuid::new_v4()),
        airports,
    };
    snapshot
        .validate()
        .map_err(|_| ClientError::MalformedWeather)?;
    Ok(snapshot)
}

fn translate_metars(
    bytes: &[u8],
    requested: &[String],
    retrieved_at: DateTime<Utc>,
) -> Result<HashMap<String, OperationalObservation<MetarObservation>>, ClientError> {
    let values = json_array(bytes)?;
    let mut translated = HashMap::new();
    for value in &values {
        let station = required_text(value, "icaoId")?.to_ascii_uppercase();
        if !requested.contains(&station) {
            continue;
        }
        let observed_at = required_timestamp(value, "obsTime")?;
        let observation = MetarObservation {
            observed_at,
            raw_text: required_text(value, "rawOb")?,
            report_type: optional_text(value, "metarType").map(|value| value.to_ascii_uppercase()),
            flight_category: optional_text(value, "fltCat").map(|value| {
                match value.to_ascii_uppercase().as_str() {
                    "VFR" => FlightCategory::Vfr,
                    "MVFR" => FlightCategory::Mvfr,
                    "IFR" => FlightCategory::Ifr,
                    "LIFR" => FlightCategory::Lifr,
                    _ => FlightCategory::Unknown,
                }
            }),
            wind_direction: wind_direction(value.get("wdir")),
            wind_speed_kt: optional_u16(value, "wspd"),
            wind_gust_kt: optional_u16(value, "wgst"),
            visibility_sm: optional_scalar_text(value, "visib"),
            temperature_c: optional_number(value, "temp"),
            dewpoint_c: optional_number(value, "dewp"),
            altimeter_hpa: optional_number(value, "altim"),
            present_weather: optional_text(value, "wxString"),
        };
        let product = OperationalObservation {
            value: observation,
            provenance: provenance(Some(observed_at), None, retrieved_at),
        };
        let replace = translated.get(&station).is_none_or(
            |existing: &OperationalObservation<MetarObservation>| {
                existing.value.observed_at < observed_at
            },
        );
        if replace {
            translated.insert(station, product);
        }
    }
    Ok(translated)
}

fn translate_tafs(
    bytes: &[u8],
    requested: &[String],
    retrieved_at: DateTime<Utc>,
) -> Result<HashMap<String, OperationalObservation<TafForecast>>, ClientError> {
    let values = json_array(bytes)?;
    let mut translated = HashMap::new();
    for value in &values {
        let station = required_text(value, "icaoId")?.to_ascii_uppercase();
        if !requested.contains(&station) {
            continue;
        }
        let issued_at = required_timestamp(value, "issueTime")?;
        let valid_from = required_timestamp(value, "validTimeFrom")?;
        let valid_to = required_timestamp(value, "validTimeTo")?;
        let product = OperationalObservation {
            value: TafForecast {
                issued_at,
                valid_from,
                valid_to,
                raw_text: required_text(value, "rawTAF")?,
            },
            provenance: provenance(Some(issued_at), Some(valid_to), retrieved_at),
        };
        let replace = translated.get(&station).is_none_or(
            |existing: &OperationalObservation<TafForecast>| existing.value.issued_at < issued_at,
        );
        if replace {
            translated.insert(station, product);
        }
    }
    Ok(translated)
}

fn provenance(
    generated_at: Option<DateTime<Utc>>,
    valid_to: Option<DateTime<Utc>>,
    retrieved_at: DateTime<Utc>,
) -> OperationalProvenance {
    OperationalProvenance {
        kind: ProvenanceKind::ExternalFact,
        provider: "aviationweather.gov".into(),
        provider_revision: Some(PROVIDER_REVISION.into()),
        generated_at,
        retrieved_at,
        transformation_version: TRANSFORMATION_VERSION,
        freshness: weather_freshness(generated_at, valid_to, retrieved_at),
    }
}

fn json_array(bytes: &[u8]) -> Result<Vec<Value>, ClientError> {
    match serde_json::from_slice(bytes).map_err(|_| ClientError::MalformedWeather)? {
        Value::Array(values) => Ok(values),
        _ => Err(ClientError::MalformedWeather),
    }
}

fn required_text(value: &Value, key: &str) -> Result<String, ClientError> {
    optional_text(value, key).ok_or(ClientError::MalformedWeather)
}

fn optional_text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_scalar_text(value: &Value, key: &str) -> Option<String> {
    let value = value.get(key)?;
    if let Some(value) = value.as_str() {
        let value = value.trim();
        return (!value.is_empty()).then(|| value.to_owned());
    }
    value.as_f64().map(|number| number.to_string())
}

fn required_timestamp(value: &Value, key: &str) -> Result<DateTime<Utc>, ClientError> {
    let value = value.get(key).ok_or(ClientError::MalformedWeather)?;
    if let Some(seconds) = value.as_i64() {
        return DateTime::from_timestamp(seconds, 0).ok_or(ClientError::MalformedWeather);
    }
    value
        .as_str()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .ok_or(ClientError::MalformedWeather)
}

fn optional_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64().filter(|value| value.is_finite())
}

fn optional_u16(value: &Value, key: &str) -> Option<u16> {
    value
        .get(key)?
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
}

fn wind_direction(value: Option<&Value>) -> Option<WindDirection> {
    let value = value?;
    if let Some(degrees) = value.as_u64().and_then(|value| u16::try_from(value).ok()) {
        return Some(WindDirection::Degrees(degrees));
    }
    value
        .as_str()
        .filter(|value| value.eq_ignore_ascii_case("VRB"))
        .map(|_| WindDirection::Variable)
}

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;

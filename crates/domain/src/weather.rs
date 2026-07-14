use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    OperationalObservation, ProvenanceKind, SnapshotFreshness, valid_code, valid_multiline_text,
    valid_text,
};

pub const WEATHER_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_WEATHER_AIRPORTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeatherSnapshotId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherSnapshot {
    pub schema_version: u32,
    pub id: WeatherSnapshotId,
    pub airports: Vec<AirportWeather>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirportWeather {
    pub station_icao: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metar: Option<OperationalObservation<MetarObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taf: Option<OperationalObservation<TafForecast>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightCategory {
    Vfr,
    Mvfr,
    Ifr,
    Lifr,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum WindDirection {
    Degrees(u16),
    Variable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetarObservation {
    pub observed_at: DateTime<Utc>,
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight_category: Option<FlightCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_direction: Option<WindDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_speed_kt: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_gust_kt: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_sm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dewpoint_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altimeter_hpa: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present_weather: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TafForecast {
    pub issued_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    pub raw_text: String,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum WeatherValidationError {
    #[error("unsupported weather snapshot schema version")]
    UnsupportedSchema,
    #[error("too many airport weather records")]
    TooManyAirports,
    #[error("invalid or duplicate weather station")]
    InvalidStation,
    #[error("invalid weather provenance")]
    InvalidProvenance,
    #[error("invalid METAR observation")]
    InvalidMetar,
    #[error("invalid TAF forecast")]
    InvalidTaf,
}

impl WeatherSnapshot {
    pub fn validate(&self) -> Result<(), WeatherValidationError> {
        if self.schema_version != WEATHER_SNAPSHOT_SCHEMA_VERSION {
            return Err(WeatherValidationError::UnsupportedSchema);
        }
        if self.airports.len() > MAX_WEATHER_AIRPORTS {
            return Err(WeatherValidationError::TooManyAirports);
        }

        let mut stations = std::collections::HashSet::new();
        for airport in &self.airports {
            if !valid_code(&airport.station_icao, 4, 4)
                || airport.station_icao != airport.station_icao.to_ascii_uppercase()
                || !stations.insert(&airport.station_icao)
            {
                return Err(WeatherValidationError::InvalidStation);
            }

            if airport.metar.as_ref().is_some_and(|product| {
                !product.provenance.is_valid()
                    || product.provenance.kind != ProvenanceKind::ExternalFact
            }) || airport.taf.as_ref().is_some_and(|product| {
                !product.provenance.is_valid()
                    || product.provenance.kind != ProvenanceKind::ExternalFact
            }) {
                return Err(WeatherValidationError::InvalidProvenance);
            }

            if airport.metar.as_ref().is_some_and(|product| {
                let value = &product.value;
                !valid_multiline_text(&value.raw_text, 2_048)
                    || value
                        .report_type
                        .as_ref()
                        .is_some_and(|report_type| !valid_code(report_type, 2, 16))
                    || value
                        .wind_direction
                        .is_some_and(|direction| matches!(direction, WindDirection::Degrees(value) if value > 360))
                    || value.wind_speed_kt.is_some_and(|speed| speed > 300)
                    || value.wind_gust_kt.is_some_and(|speed| speed > 400)
                    || value
                        .visibility_sm
                        .as_ref()
                        .is_some_and(|visibility| !valid_text(visibility, 24))
                    || [value.temperature_c, value.dewpoint_c]
                        .into_iter()
                        .flatten()
                        .any(|temperature| {
                            !temperature.is_finite() || !(-150.0..=100.0).contains(&temperature)
                        })
                    || value.altimeter_hpa.is_some_and(|altimeter| {
                        !altimeter.is_finite() || !(800.0..=1_200.0).contains(&altimeter)
                    })
                    || value
                        .present_weather
                        .as_ref()
                        .is_some_and(|weather| !valid_text(weather, 128))
            }) {
                return Err(WeatherValidationError::InvalidMetar);
            }

            if airport.taf.as_ref().is_some_and(|product| {
                let value = &product.value;
                value.valid_from >= value.valid_to
                    || !valid_multiline_text(&value.raw_text, 32 * 1_024)
            }) {
                return Err(WeatherValidationError::InvalidTaf);
            }
        }

        Ok(())
    }
}

pub fn weather_freshness(
    generated_at: Option<DateTime<Utc>>,
    valid_to: Option<DateTime<Utc>>,
    retrieved_at: DateTime<Utc>,
) -> SnapshotFreshness {
    if let Some(valid_to) = valid_to {
        return if valid_to < retrieved_at {
            SnapshotFreshness::Stale
        } else {
            SnapshotFreshness::Current
        };
    }
    match generated_at {
        Some(generated_at) if retrieved_at.signed_duration_since(generated_at).num_hours() > 2 => {
            SnapshotFreshness::Stale
        }
        Some(_) => SnapshotFreshness::Current,
        None => SnapshotFreshness::Unknown,
    }
}

#[cfg(test)]
#[path = "tests/weather.rs"]
mod tests;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    OperationalObservation, OperationalProvenance, ProvenanceKind, SnapshotFreshness, valid_code,
    valid_multiline_text, valid_text,
};

pub const WEATHER_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_WEATHER_AIRPORTS: usize = 10;
pub const GLOBAL_WEATHER_LAYER_SCHEMA_VERSION: u32 = 1;
pub const MAX_GLOBAL_WEATHER_GRID_POINTS: usize = 512;
pub const MAX_GLOBAL_WEATHER_RASTER_TILES: usize = 16;
pub const MAX_GLOBAL_WEATHER_RASTER_TILE_BYTES: usize = 192 * 1_024;
pub const MAX_GLOBAL_WEATHER_RASTER_BYTES: usize = 640 * 1_024;
pub const MAX_GLOBAL_WEATHER_TILE_ZOOM: u8 = 8;
pub const GLOBAL_WEATHER_RASTER_TILE_DIMENSION: u32 = 256;

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

/// A provider-neutral weather product rendered entirely by the WyrmGrid host.
///
/// Plugins translate raw provider payloads into this bounded contract. They
/// cannot supply styles, scripts, URLs, or executable rendering instructions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalWeatherLayerSnapshot {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub data: GlobalWeatherLayerData,
    pub provenance: OperationalProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GlobalWeatherLayerData {
    Grid {
        points: Vec<GlobalWeatherGridPoint>,
    },
    RasterTiles {
        frame_time: DateTime<Utc>,
        tiles: Vec<GlobalWeatherRasterTile>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherCondition {
    Clear,
    Cloud,
    Rain,
    Snow,
    Convective,
    Obscuration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalWeatherGridPoint {
    pub id: String,
    pub location: crate::Coordinates,
    /// The UTC time represented by this model sample. Older plugins may omit
    /// it; callers must then treat the point as current context, not a
    /// time-matched forecast.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    pub condition: WeatherCondition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precipitation_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_cover_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_direction_degrees: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_speed_kt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_weather_code: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalWeatherRasterTile {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
    /// A base64-encoded PNG. The host validates the decoded bytes before use.
    pub png_base64: String,
    /// An optional provider coverage mask. Transparent pixels are covered and
    /// opaque pixels represent unavailable RADAR coverage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_png_base64: Option<String>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GlobalWeatherValidationError {
    #[error("unsupported global weather layer schema version")]
    UnsupportedSchema,
    #[error("invalid global weather layer identity")]
    InvalidIdentity,
    #[error("invalid global weather provenance")]
    InvalidProvenance,
    #[error("global weather layer has no data")]
    Empty,
    #[error("global weather grid exceeds its point limit")]
    TooManyGridPoints,
    #[error("global weather grid contains an invalid point")]
    InvalidGridPoint,
    #[error("global weather raster exceeds its tile or byte limit")]
    RasterTooLarge,
    #[error("global weather raster contains an invalid tile")]
    InvalidRasterTile,
}

impl GlobalWeatherLayerSnapshot {
    pub fn validate(&self) -> Result<(), GlobalWeatherValidationError> {
        if self.schema_version != GLOBAL_WEATHER_LAYER_SCHEMA_VERSION {
            return Err(GlobalWeatherValidationError::UnsupportedSchema);
        }
        if !valid_weather_identifier(&self.id) || !valid_text(&self.title, 120) {
            return Err(GlobalWeatherValidationError::InvalidIdentity);
        }
        if !self.provenance.is_valid()
            || !matches!(
                self.provenance.kind,
                ProvenanceKind::ExternalFact | ProvenanceKind::ExternalCalculation
            )
        {
            return Err(GlobalWeatherValidationError::InvalidProvenance);
        }

        match &self.data {
            GlobalWeatherLayerData::Grid { points } => validate_global_grid(points),
            GlobalWeatherLayerData::RasterTiles { tiles, .. } => {
                validate_global_raster_tiles(tiles)
            }
        }
    }
}

fn validate_global_grid(
    points: &[GlobalWeatherGridPoint],
) -> Result<(), GlobalWeatherValidationError> {
    if points.is_empty() {
        return Err(GlobalWeatherValidationError::Empty);
    }
    if points.len() > MAX_GLOBAL_WEATHER_GRID_POINTS {
        return Err(GlobalWeatherValidationError::TooManyGridPoints);
    }
    let mut ids = std::collections::HashSet::with_capacity(points.len());
    for point in points {
        if !valid_weather_identifier(&point.id)
            || !ids.insert(point.id.as_str())
            || !point.location.is_valid()
            || point
                .temperature_c
                .is_some_and(|value| !value.is_finite() || !(-150.0..=100.0).contains(&value))
            || point
                .precipitation_mm
                .is_some_and(|value| !value.is_finite() || !(0.0..=1_000.0).contains(&value))
            || point
                .cloud_cover_percent
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || point
                .wind_direction_degrees
                .is_some_and(|value| !value.is_finite() || !(0.0..=360.0).contains(&value))
            || point
                .wind_speed_kt
                .is_some_and(|value| !value.is_finite() || !(0.0..=500.0).contains(&value))
        {
            return Err(GlobalWeatherValidationError::InvalidGridPoint);
        }
    }
    Ok(())
}

fn validate_global_raster_tiles(
    tiles: &[GlobalWeatherRasterTile],
) -> Result<(), GlobalWeatherValidationError> {
    if tiles.is_empty() {
        return Err(GlobalWeatherValidationError::Empty);
    }
    if tiles.len() > MAX_GLOBAL_WEATHER_RASTER_TILES {
        return Err(GlobalWeatherValidationError::RasterTooLarge);
    }

    let mut addresses = std::collections::HashSet::with_capacity(tiles.len());
    let mut total_bytes = 0_usize;
    for tile in tiles {
        let maximum_coordinate = 1_u32
            .checked_shl(u32::from(tile.zoom))
            .ok_or(GlobalWeatherValidationError::InvalidRasterTile)?;
        if tile.zoom > MAX_GLOBAL_WEATHER_TILE_ZOOM
            || tile.x >= maximum_coordinate
            || tile.y >= maximum_coordinate
            || !addresses.insert((tile.zoom, tile.x, tile.y))
        {
            return Err(GlobalWeatherValidationError::InvalidRasterTile);
        }
        for encoded in
            std::iter::once(tile.png_base64.as_str()).chain(tile.coverage_png_base64.as_deref())
        {
            let bytes = BASE64_STANDARD
                .decode(encoded)
                .map_err(|_| GlobalWeatherValidationError::InvalidRasterTile)?;
            if bytes.len() > MAX_GLOBAL_WEATHER_RASTER_TILE_BYTES {
                return Err(GlobalWeatherValidationError::RasterTooLarge);
            }
            if !valid_weather_png(&bytes) {
                return Err(GlobalWeatherValidationError::InvalidRasterTile);
            }
            total_bytes = total_bytes.saturating_add(bytes.len());
            if total_bytes > MAX_GLOBAL_WEATHER_RASTER_BYTES {
                return Err(GlobalWeatherValidationError::RasterTooLarge);
            }
        }
    }
    Ok(())
}

fn valid_weather_png(bytes: &[u8]) -> bool {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    const IEND_PREFIX: &[u8; 8] = b"\0\0\0\0IEND";
    if bytes.len() < 45
        || !bytes.starts_with(SIGNATURE)
        || bytes.get(8..12) != Some(&13_u32.to_be_bytes())
        || bytes.get(12..16) != Some(b"IHDR")
        || bytes.get(bytes.len() - 12..bytes.len() - 4) != Some(IEND_PREFIX)
    {
        return false;
    }
    let Some(width) = bytes
        .get(16..20)
        .and_then(|value| <[u8; 4]>::try_from(value).ok())
        .map(u32::from_be_bytes)
    else {
        return false;
    };
    let Some(height) = bytes
        .get(20..24)
        .and_then(|value| <[u8; 4]>::try_from(value).ok())
        .map(u32::from_be_bytes)
    else {
        return false;
    };
    width == GLOBAL_WEATHER_RASTER_TILE_DIMENSION
        && height == GLOBAL_WEATHER_RASTER_TILE_DIMENSION
        && bytes
            .get(24)
            .is_some_and(|depth| matches!(depth, 1 | 2 | 4 | 8 | 16))
        && bytes
            .get(25)
            .is_some_and(|colour| matches!(colour, 0 | 2 | 3 | 4 | 6))
        && bytes.get(26) == Some(&0)
        && bytes.get(27) == Some(&0)
        && bytes.get(28).is_some_and(|interlace| *interlace <= 1)
}

fn valid_weather_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
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

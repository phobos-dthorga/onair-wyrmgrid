//! Versioned, language-neutral contracts for out-of-process plugins.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Write};
use thiserror::Error;
use wyrmgrid_domain::{AircraftSummary, Coordinates, Provenance};

pub const PLUGIN_API_VERSION: u32 = 1;
pub const PLUGIN_PROTOCOL_VERSION: u32 = 1;
pub const CHART_SCHEMA_VERSION: u32 = 1;
pub const MAX_CHART_SERIES: usize = 12;
pub const MAX_CHART_POINTS_PER_SERIES: usize = 10_000;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_MAP_LAYERS_PER_PLUGIN: usize = 16;
pub const MAX_MAP_POINTS_PER_LAYER: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    OnAirCompanyRead,
    OnAirFleetRead,
    OnAirJobsRead,
    OnAirFinanceRead,
    MapLayersPublish,
    ChartsPublish,
    NotificationsCreate,
    PluginStorage,
    SimulatorTelemetryRead,
    ExternalNetwork,
}

impl Permission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnAirCompanyRead => "on_air_company_read",
            Self::OnAirFleetRead => "on_air_fleet_read",
            Self::OnAirJobsRead => "on_air_jobs_read",
            Self::OnAirFinanceRead => "on_air_finance_read",
            Self::MapLayersPublish => "map_layers_publish",
            Self::ChartsPublish => "charts_publish",
            Self::NotificationsCreate => "notifications_create",
            Self::PluginStorage => "plugin_storage",
            Self::SimulatorTelemetryRead => "simulator_telemetry_read",
            Self::ExternalNetwork => "external_network",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value {
            "on_air_company_read" => Some(Self::OnAirCompanyRead),
            "on_air_fleet_read" => Some(Self::OnAirFleetRead),
            "on_air_jobs_read" => Some(Self::OnAirJobsRead),
            "on_air_finance_read" => Some(Self::OnAirFinanceRead),
            "map_layers_publish" => Some(Self::MapLayersPublish),
            "charts_publish" => Some(Self::ChartsPublish),
            "notifications_create" => Some(Self::NotificationsCreate),
            "plugin_storage" => Some(Self::PluginStorage),
            "simulator_telemetry_read" => Some(Self::SimulatorTelemetryRead),
            "external_network" => Some(Self::ExternalNetwork),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntime {
    Python,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolEnvelope<T> {
    pub protocol_version: u32,
    pub sequence: u64,
    pub payload: T,
}

impl<T> ProtocolEnvelope<T> {
    pub fn new(sequence: u64, payload: T) -> Self {
        Self {
            protocol_version: PLUGIN_PROTOCOL_VERSION,
            sequence,
            payload,
        }
    }

    pub fn validate_header(&self) -> Result<(), EnvelopeError> {
        if self.protocol_version != PLUGIN_PROTOCOL_VERSION {
            return Err(EnvelopeError::UnsupportedProtocolVersion {
                found: self.protocol_version,
                supported: PLUGIN_PROTOCOL_VERSION,
            });
        }
        if self.sequence == 0 {
            return Err(EnvelopeError::InvalidSequence);
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvelopeError {
    #[error("unsupported plugin protocol version {found}; host supports {supported}")]
    UnsupportedProtocolVersion { found: u32, supported: u32 },
    #[error("plugin message sequence must be greater than zero")]
    InvalidSequence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMessage {
    Hello {
        host_version: String,
        plugin_id: String,
        granted_permissions: Vec<Permission>,
    },
    FleetSnapshot {
        snapshot: PluginFleetSnapshot,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginMessage {
    Ready { plugin_id: String, api_version: u32 },
    PublishMapLayer { layer: MapLayerSpec },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCompany {
    pub name: String,
    pub airline_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSnapshotAvailability {
    Live,
    Cached,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginFleetSnapshot {
    pub company: PluginCompany,
    pub aircraft: Vec<AircraftSummary>,
    pub provenance: Provenance,
    pub availability: PluginSnapshotAvailability,
}

/// A data-only map contract. The host owns rendering and interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapLayerSpec {
    pub id: String,
    pub title: String,
    pub points: Vec<MapPoint>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPoint {
    pub id: String,
    pub label: String,
    pub location: Coordinates,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MapLayerError {
    #[error("map layer id and title must be valid, bounded text")]
    InvalidIdentity,
    #[error("map layer exceeds the maximum of {maximum} points")]
    TooManyPoints { maximum: usize },
    #[error("map point ids must be non-empty and unique")]
    InvalidPointId,
    #[error("map point labels must be valid, bounded text")]
    InvalidPointLabel,
    #[error("map point coordinates must be valid WGS84 coordinates")]
    InvalidCoordinates,
    #[error("map layer provenance source must be valid, bounded text")]
    InvalidProvenanceSource,
}

impl MapLayerSpec {
    pub fn validate(&self) -> Result<(), MapLayerError> {
        if !valid_identifier(&self.id) || !valid_text(&self.title, 120) {
            return Err(MapLayerError::InvalidIdentity);
        }
        if self.points.len() > MAX_MAP_POINTS_PER_LAYER {
            return Err(MapLayerError::TooManyPoints {
                maximum: MAX_MAP_POINTS_PER_LAYER,
            });
        }
        if !valid_text(&self.provenance.source, 160) {
            return Err(MapLayerError::InvalidProvenanceSource);
        }

        let mut point_ids = HashSet::with_capacity(self.points.len());
        for point in &self.points {
            if !valid_identifier(&point.id) || !point_ids.insert(point.id.as_str()) {
                return Err(MapLayerError::InvalidPointId);
            }
            if !valid_text(&point.label, 200) {
                return Err(MapLayerError::InvalidPointLabel);
            }
            if !point.location.is_valid() {
                return Err(MapLayerError::InvalidCoordinates);
            }
        }
        Ok(())
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
}

fn valid_text(value: &str, maximum_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum_bytes && !value.chars().any(char::is_control)
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("plugin stream closed")]
    Closed,
    #[error("plugin frame header is incomplete")]
    TruncatedHeader,
    #[error("plugin frame exceeds the {maximum}-byte limit")]
    TooLarge { maximum: usize },
    #[error("plugin frame is empty")]
    Empty,
    #[error("plugin stream I/O failed")]
    Io(#[source] std::io::Error),
    #[error("plugin message could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("plugin message could not be decoded")]
    Decode(#[source] serde_json::Error),
}

pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, message: &T) -> Result<(), FrameError> {
    let payload = serde_json::to_vec(message).map_err(FrameError::Encode)?;
    if payload.is_empty() {
        return Err(FrameError::Empty);
    }
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            maximum: MAX_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::TooLarge {
        maximum: MAX_FRAME_BYTES,
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .map_err(FrameError::Io)?;
    writer.write_all(&payload).map_err(FrameError::Io)?;
    writer.flush().map_err(FrameError::Io)
}

pub fn read_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T, FrameError> {
    let mut header = [0_u8; 4];
    match reader.read(&mut header[..1]) {
        Ok(0) => return Err(FrameError::Closed),
        Ok(1) => {}
        Ok(_) => unreachable!("one-byte read returned more than one byte"),
        Err(error) => return Err(FrameError::Io(error)),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::UnexpectedEof => FrameError::TruncatedHeader,
            _ => FrameError::Io(error),
        })?;

    let length = u32::from_be_bytes(header) as usize;
    if length == 0 {
        return Err(FrameError::Empty);
    }
    if length > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            maximum: MAX_FRAME_BYTES,
        });
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).map_err(FrameError::Io)?;
    serde_json::from_slice(&payload).map_err(FrameError::Decode)
}

/// A deliberately small chart contract shared by first- and third-party views.
///
/// The host owns rendering, colours, accessibility, and interaction. Plugins
/// provide validated data and presentation intent, never executable chart code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSpec {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: ChartKind,
    #[serde(default)]
    pub category_axis_label: Option<String>,
    #[serde(default)]
    pub value_axis_label: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    pub series: Vec<ChartSeries>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Line,
    Area,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSeries {
    pub id: String,
    pub label: String,
    pub points: Vec<ChartPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartPoint {
    pub category: String,
    pub value: f64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChartError {
    #[error("unsupported chart schema version {found}; host supports {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    #[error("chart id and title must not be empty")]
    MissingIdentity,
    #[error("chart must contain at least one series")]
    MissingSeries,
    #[error("chart exceeds the maximum of {maximum} series")]
    TooManySeries { maximum: usize },
    #[error("chart series ids must be non-empty and unique")]
    InvalidSeriesId,
    #[error("chart series must contain at least one point")]
    MissingPoints,
    #[error("chart series exceeds the maximum of {maximum} points")]
    TooManyPoints { maximum: usize },
    #[error("chart point categories must not be empty")]
    MissingCategory,
    #[error("chart point values must be finite")]
    NonFiniteValue,
    #[error("chart provenance source must not be empty")]
    MissingProvenanceSource,
}

impl ChartSpec {
    pub fn validate(&self) -> Result<(), ChartError> {
        if self.schema_version != CHART_SCHEMA_VERSION {
            return Err(ChartError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: CHART_SCHEMA_VERSION,
            });
        }

        if self.id.trim().is_empty() || self.title.trim().is_empty() {
            return Err(ChartError::MissingIdentity);
        }
        if self.series.is_empty() {
            return Err(ChartError::MissingSeries);
        }
        if self.series.len() > MAX_CHART_SERIES {
            return Err(ChartError::TooManySeries {
                maximum: MAX_CHART_SERIES,
            });
        }
        if self.provenance.source.trim().is_empty() {
            return Err(ChartError::MissingProvenanceSource);
        }

        let mut series_ids = HashSet::with_capacity(self.series.len());
        for series in &self.series {
            if series.id.trim().is_empty() || !series_ids.insert(series.id.as_str()) {
                return Err(ChartError::InvalidSeriesId);
            }
            if series.points.is_empty() {
                return Err(ChartError::MissingPoints);
            }
            if series.points.len() > MAX_CHART_POINTS_PER_SERIES {
                return Err(ChartError::TooManyPoints {
                    maximum: MAX_CHART_POINTS_PER_SERIES,
                });
            }
            for point in &series.points {
                if point.category.trim().is_empty() {
                    return Err(ChartError::MissingCategory);
                }
                if !point.value.is_finite() {
                    return Err(ChartError::NonFiniteValue);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub author: String,
    #[serde(default)]
    pub runtime: Option<PluginRuntime>,
    pub entry_point: String,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("plugin id must use reverse-domain notation")]
    InvalidId,
    #[error("plugin name, version, and author must be valid, bounded text")]
    InvalidMetadata,
    #[error("unsupported plugin API version {found}; host supports {supported}")]
    UnsupportedApiVersion { found: u32, supported: u32 },
    #[error("plugin entry point must be a relative path")]
    UnsafeEntryPoint,
    #[error("plugin permissions must not contain duplicates")]
    DuplicatePermissions,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        let id_segments = self.id.split('.').collect::<Vec<_>>();
        if id_segments.len() < 3
            || id_segments.iter().any(|segment| {
                segment.is_empty()
                    || segment.len() > 63
                    || !segment.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '-'
                    })
                    || !segment.chars().next().is_some_and(|character| {
                        character.is_ascii_lowercase() || character.is_ascii_digit()
                    })
                    || !segment.chars().last().is_some_and(|character| {
                        character.is_ascii_lowercase() || character.is_ascii_digit()
                    })
            })
            || self.id.len() > 255
        {
            return Err(ManifestError::InvalidId);
        }

        let semantic_version = self.version.split('.').collect::<Vec<_>>();
        if !valid_text(&self.name, 120)
            || !valid_text(&self.author, 120)
            || semantic_version.len() != 3
            || semantic_version
                .iter()
                .any(|component| component.parse::<u64>().is_err())
        {
            return Err(ManifestError::InvalidMetadata);
        }

        if self.api_version != PLUGIN_API_VERSION {
            return Err(ManifestError::UnsupportedApiVersion {
                found: self.api_version,
                supported: PLUGIN_API_VERSION,
            });
        }

        let path = std::path::Path::new(&self.entry_point);
        if self.entry_point.trim().is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(ManifestError::UnsafeEntryPoint);
        }

        if self.permissions.iter().collect::<HashSet<_>>().len() != self.permissions.len() {
            return Err(ManifestError::DuplicatePermissions);
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;

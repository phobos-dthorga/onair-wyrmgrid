//! Versioned, language-neutral contracts for out-of-process plugins.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use wyrmgrid_domain::Provenance;

pub const PLUGIN_API_VERSION: u32 = 1;
pub const CHART_SCHEMA_VERSION: u32 = 1;
pub const MAX_CHART_SERIES: usize = 12;
pub const MAX_CHART_POINTS_PER_SERIES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub author: String,
    pub entry_point: String,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("plugin id must use reverse-domain notation")]
    InvalidId,
    #[error("unsupported plugin API version {found}; host supports {supported}")]
    UnsupportedApiVersion { found: u32, supported: u32 },
    #[error("plugin entry point must be a relative path")]
    UnsafeEntryPoint,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.split('.').count() < 3
            || self.id.chars().any(|character| {
                !(character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '.' | '-'))
            })
        {
            return Err(ManifestError::InvalidId);
        }

        if self.api_version != PLUGIN_API_VERSION {
            return Err(ManifestError::UnsupportedApiVersion {
                found: self.api_version,
                supported: PLUGIN_API_VERSION,
            });
        }

        let path = std::path::Path::new(&self.entry_point);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(ManifestError::UnsafeEntryPoint);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest {
            id: "org.wyrmgrid.example.idle-aircraft".into(),
            name: "Idle Aircraft".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION,
            author: "Example Developer".into(),
            entry_point: "src/main.py".into(),
            permissions: vec![Permission::OnAirFleetRead, Permission::MapLayersPublish],
        }
    }

    #[test]
    fn accepts_a_safe_manifest() {
        assert_eq!(manifest().validate(), Ok(()));
    }

    #[test]
    fn rejects_parent_directory_entry_points() {
        let mut candidate = manifest();
        candidate.entry_point = "../outside.py".into();
        assert_eq!(candidate.validate(), Err(ManifestError::UnsafeEntryPoint));
    }

    #[test]
    fn validates_the_version_one_chart_fixture() {
        let chart: ChartSpec =
            serde_json::from_str(include_str!("../../../schemas/fixtures/chart-spec-v1.json"))
                .expect("chart fixture should deserialize");

        assert_eq!(chart.validate(), Ok(()));
        assert_eq!(chart.kind, ChartKind::Area);
        assert_eq!(chart.series.len(), 1);
    }

    #[test]
    fn rejects_non_finite_chart_values() {
        let mut chart: ChartSpec =
            serde_json::from_str(include_str!("../../../schemas/fixtures/chart-spec-v1.json"))
                .expect("chart fixture should deserialize");
        chart.series[0].points[0].value = f64::NAN;

        assert_eq!(chart.validate(), Err(ChartError::NonFiniteValue));
    }

    #[test]
    fn rejects_oversized_chart_series() {
        let mut chart: ChartSpec =
            serde_json::from_str(include_str!("../../../schemas/fixtures/chart-spec-v1.json"))
                .expect("chart fixture should deserialize");
        chart.series[0].points = (0..=MAX_CHART_POINTS_PER_SERIES)
            .map(|index| ChartPoint {
                category: index.to_string(),
                value: index as f64,
            })
            .collect();

        assert_eq!(
            chart.validate(),
            Err(ChartError::TooManyPoints {
                maximum: MAX_CHART_POINTS_PER_SERIES,
            })
        );
    }
}

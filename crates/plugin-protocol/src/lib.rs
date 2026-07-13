//! Versioned, language-neutral contracts for out-of-process plugins.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PLUGIN_API_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    OnAirCompanyRead,
    OnAirFleetRead,
    OnAirJobsRead,
    OnAirFinanceRead,
    MapLayersPublish,
    NotificationsCreate,
    PluginStorage,
    SimulatorTelemetryRead,
    ExternalNetwork,
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
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ProvenanceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotFreshness {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalProvenance {
    pub kind: ProvenanceKind,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<DateTime<Utc>>,
    pub retrieved_at: DateTime<Utc>,
    pub transformation_version: u32,
    pub freshness: SnapshotFreshness,
}

impl OperationalProvenance {
    pub fn is_valid(&self) -> bool {
        valid_text(&self.provider, 64)
            && self
                .provider_revision
                .as_ref()
                .is_none_or(|revision| valid_text(revision, 64))
            && self.transformation_version > 0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationalObservation<T> {
    pub value: T,
    pub provenance: OperationalProvenance,
}

pub(crate) fn valid_code(value: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&value.len())
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '/'
        })
}

pub(crate) fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

pub(crate) fn valid_multiline_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

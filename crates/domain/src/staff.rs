use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use crate::AirportSummary;

pub const STAFF_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_STAFF_PER_SNAPSHOT: usize = 1_024;
pub const MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StaffMemberId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StaffQualificationId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AircraftClassId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AircraftClassQualification {
    pub id: StaffQualificationId,
    pub aircraft_class_id: AircraftClassId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaffMemberSummary {
    pub id: StaffMemberId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_airport: Option<AirportSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_airport: Option<AirportSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub busy_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_online: Option<bool>,
    pub class_qualifications: Vec<AircraftClassQualification>,
}

impl StaffMemberSummary {
    pub fn validate(&self) -> Result<(), StaffValidationError> {
        if !valid_optional_text(self.display_name.as_deref(), 120)
            || !valid_optional_text(self.avatar_reference.as_deref(), 255)
            || !self
                .category_code
                .is_none_or(|code| (0..=4).contains(&code))
            || !self.status_code.is_none_or(|code| (0..=11).contains(&code))
            || self.class_qualifications.len() > MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER
        {
            return Err(StaffValidationError::InvalidMember);
        }

        for airport in [&self.current_airport, &self.home_airport]
            .into_iter()
            .flatten()
        {
            if !airport.location.is_none_or(|location| location.is_valid()) {
                return Err(StaffValidationError::InvalidAirport);
            }
        }

        let mut qualification_ids = HashSet::new();
        let mut aircraft_class_ids = HashSet::new();
        for qualification in &self.class_qualifications {
            if !qualification_ids.insert(&qualification.id)
                || !aircraft_class_ids.insert(&qualification.aircraft_class_id)
                || !valid_optional_text(qualification.short_name.as_deref(), 64)
                || !valid_optional_text(qualification.name.as_deref(), 160)
            {
                return Err(StaffValidationError::InvalidQualification);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaffSnapshot {
    pub schema_version: u32,
    pub staff: Vec<StaffMemberSummary>,
}

impl StaffSnapshot {
    pub fn validate(&self) -> Result<(), StaffValidationError> {
        if self.schema_version != STAFF_SNAPSHOT_SCHEMA_VERSION
            || self.staff.len() > MAX_STAFF_PER_SNAPSHOT
        {
            return Err(StaffValidationError::InvalidSnapshot);
        }
        let mut ids = HashSet::new();
        for member in &self.staff {
            if !ids.insert(&member.id) {
                return Err(StaffValidationError::InvalidSnapshot);
            }
            member.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum StaffValidationError {
    #[error("invalid staff snapshot")]
    InvalidSnapshot,
    #[error("invalid staff member")]
    InvalidMember,
    #[error("invalid staff airport")]
    InvalidAirport,
    #[error("invalid staff qualification")]
    InvalidQualification,
}

fn valid_optional_text(value: Option<&str>, maximum: usize) -> bool {
    value.is_none_or(|value| {
        !value.trim().is_empty()
            && value.len() <= maximum
            && !value.chars().any(|character| character.is_control())
    })
}

#[cfg(test)]
#[path = "tests/staff.rs"]
mod tests;

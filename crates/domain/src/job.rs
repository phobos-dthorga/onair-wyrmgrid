use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use crate::AirportSummary;

pub const JOB_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_JOBS_PER_SNAPSHOT: usize = 512;
pub const MAX_JOB_LEGS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobLegId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLegKind {
    Cargo,
    Passengers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobLeg {
    pub id: JobLegId,
    pub sequence: u32,
    pub kind: JobLegKind,
    pub departure: Option<AirportSummary>,
    pub destination: Option<AirportSummary>,
    pub current_airport: Option<AirportSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_weight_lb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passengers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_pay: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub legs: Vec<JobLeg>,
}

impl JobSummary {
    pub fn cargo_weight_lb(&self) -> Option<f64> {
        let weights = self
            .legs
            .iter()
            .filter_map(|leg| leg.cargo_weight_lb)
            .collect::<Vec<_>>();
        (!weights.is_empty()).then(|| weights.into_iter().sum())
    }

    pub fn passenger_count(&self) -> Option<u32> {
        let passengers = self
            .legs
            .iter()
            .filter_map(|leg| leg.passengers)
            .collect::<Vec<_>>();
        (!passengers.is_empty()).then(|| passengers.into_iter().fold(0_u32, u32::saturating_add))
    }

    pub fn route(&self) -> Option<(&AirportSummary, &AirportSummary)> {
        let first = self.legs.first()?.departure.as_ref()?;
        let last = self.legs.last()?.destination.as_ref()?;
        Some((first, last))
    }

    pub fn validate(&self) -> Result<(), JobValidationError> {
        if self.legs.is_empty() || self.legs.len() > MAX_JOB_LEGS {
            return Err(JobValidationError::InvalidLegs);
        }
        if !self
            .reported_pay
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        {
            return Err(JobValidationError::InvalidPay);
        }
        if !valid_optional_text(self.mission_type.as_deref(), 120)
            || !valid_optional_text(self.description.as_deref(), 1_024)
        {
            return Err(JobValidationError::InvalidText);
        }

        let mut ids = HashSet::new();
        let mut previous_sequence = None;
        for leg in &self.legs {
            if !ids.insert(&leg.id) || previous_sequence.is_some_and(|value| leg.sequence <= value)
            {
                return Err(JobValidationError::InvalidLegs);
            }
            previous_sequence = Some(leg.sequence);
            if !leg
                .cargo_weight_lb
                .is_none_or(|value| value.is_finite() && value >= 0.0)
                || !leg
                    .distance_nm
                    .is_none_or(|value| value.is_finite() && value >= 0.0)
                || !valid_optional_text(leg.description.as_deref(), 512)
            {
                return Err(JobValidationError::InvalidLegs);
            }
            for airport in [&leg.departure, &leg.destination, &leg.current_airport]
                .into_iter()
                .flatten()
            {
                if !airport.location.is_none_or(|location| location.is_valid()) {
                    return Err(JobValidationError::InvalidAirport);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub schema_version: u32,
    pub jobs: Vec<JobSummary>,
}

impl JobSnapshot {
    pub fn validate(&self) -> Result<(), JobValidationError> {
        if self.schema_version != JOB_SNAPSHOT_SCHEMA_VERSION
            || self.jobs.len() > MAX_JOBS_PER_SNAPSHOT
        {
            return Err(JobValidationError::InvalidSnapshot);
        }
        let mut ids = HashSet::new();
        for job in &self.jobs {
            if !ids.insert(&job.id) {
                return Err(JobValidationError::InvalidSnapshot);
            }
            job.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum JobValidationError {
    #[error("invalid job snapshot")]
    InvalidSnapshot,
    #[error("invalid job legs")]
    InvalidLegs,
    #[error("invalid reported pay")]
    InvalidPay,
    #[error("invalid job text")]
    InvalidText,
    #[error("invalid job airport")]
    InvalidAirport,
}

fn valid_optional_text(value: Option<&str>, maximum: usize) -> bool {
    value.is_none_or(|value| {
        !value.trim().is_empty()
            && value.len() <= maximum
            && !value.chars().any(|character| character.is_control())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_the_version_one_fixture() {
        let snapshot: JobSnapshot = serde_json::from_str(include_str!(
            "../../../schemas/fixtures/job-snapshot-v1.json"
        ))
        .expect("job fixture should deserialize");
        snapshot.validate().expect("job fixture should validate");
        assert_eq!(snapshot.jobs[0].cargo_weight_lb(), Some(4_000.0));
        assert_eq!(snapshot.jobs[1].passenger_count(), Some(8));
    }

    #[test]
    fn rejects_duplicate_jobs_and_non_monotonic_legs() {
        let mut snapshot: JobSnapshot = serde_json::from_str(include_str!(
            "../../../schemas/fixtures/job-snapshot-v1.json"
        ))
        .unwrap();
        snapshot.jobs.push(snapshot.jobs[0].clone());
        assert_eq!(
            snapshot.validate(),
            Err(JobValidationError::InvalidSnapshot)
        );

        snapshot.jobs.pop();
        snapshot.jobs[0].legs[1].sequence = 0;
        assert_eq!(snapshot.validate(), Err(JobValidationError::InvalidLegs));
    }
}

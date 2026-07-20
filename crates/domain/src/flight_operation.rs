use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    AircraftSummary, AirportSummary, CompanyId, FlightPlanSnapshot, JobLegId, JobLegKind,
    JobSummary, Observed, OperationalObservation, ProvenanceKind, valid_code, valid_text,
};

pub const FLIGHT_OPERATION_SCHEMA_VERSION: u32 = 1;
pub const FLIGHT_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const REVIEWED_AIRCRAFT_ASSIGNMENT_SCHEMA_VERSION: u32 = 1;
pub const REVIEWED_AIRCRAFT_ASSIGNMENT_REVISION_SCHEMA_VERSION: u32 = 1;
pub const MAX_FLIGHT_MANIFEST_LEGS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlightOperationId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationRevisionReason {
    Initial,
    PlanChanged,
    JobChanged,
    PlanAndJobChanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewedAircraftAssignment {
    pub schema_version: u32,
    pub company_id: CompanyId,
    pub aircraft: Observed<AircraftSummary>,
}

impl ReviewedAircraftAssignment {
    pub fn validate(&self) -> Result<(), FlightOperationValidationError> {
        let aircraft = &self.aircraft.value;
        let airport_valid = aircraft.current_airport.as_ref().is_none_or(|airport| {
            !airport.id.0.is_nil()
                && airport
                    .icao
                    .as_deref()
                    .is_none_or(|icao| valid_code(icao, 3, 8))
                && airport
                    .name
                    .as_deref()
                    .is_none_or(|name| valid_text(name, 160))
                && airport.location.is_none_or(|location| location.is_valid())
        });
        if self.schema_version != REVIEWED_AIRCRAFT_ASSIGNMENT_SCHEMA_VERSION
            || self.company_id.0.is_nil()
            || aircraft.id.0.is_nil()
            || aircraft
                .registration
                .as_deref()
                .is_some_and(|value| !valid_text(value, 64))
            || aircraft
                .model
                .as_deref()
                .is_some_and(|value| !valid_text(value, 160))
            || aircraft
                .location
                .is_some_and(|location| !location.is_valid())
            || !airport_valid
            || self.aircraft.provenance.kind != ProvenanceKind::OnAirFact
            || !valid_text(&self.aircraft.provenance.source, 64)
        {
            return Err(FlightOperationValidationError::InvalidAircraftAssignment);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewedAircraftAssignmentRevisionReason {
    Assigned,
    Reassigned,
    Cleared,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewedAircraftAssignmentRevision {
    pub schema_version: u32,
    pub operation_id: FlightOperationId,
    pub revision: u32,
    pub reason: ReviewedAircraftAssignmentRevisionReason,
    pub reviewed_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignment: Option<ReviewedAircraftAssignment>,
}

impl ReviewedAircraftAssignmentRevision {
    pub fn validate(&self) -> Result<(), FlightOperationValidationError> {
        if self.schema_version != REVIEWED_AIRCRAFT_ASSIGNMENT_REVISION_SCHEMA_VERSION
            || self.operation_id.0.is_nil()
            || self.revision == 0
            || (self.revision == 1)
                != (self.reason == ReviewedAircraftAssignmentRevisionReason::Assigned)
            || matches!(
                (self.reason, self.assignment.is_some()),
                (ReviewedAircraftAssignmentRevisionReason::Cleared, true)
                    | (
                        ReviewedAircraftAssignmentRevisionReason::Assigned
                            | ReviewedAircraftAssignmentRevisionReason::Reassigned,
                        false
                    )
            )
        {
            return Err(FlightOperationValidationError::InvalidAircraftAssignment);
        }
        if let Some(assignment) = &self.assignment {
            assignment.validate()?;
            if assignment.aircraft.provenance.observed_at > self.reviewed_at {
                return Err(FlightOperationValidationError::InvalidAircraftAssignment);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestUnavailableField {
    PassengerCount,
    FreightWeight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestPassengerLoad {
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFreightLoad {
    pub weight_lb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightManifestLeg {
    pub source_job_leg_id: JobLegId,
    pub sequence: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure: Option<AirportSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<AirportSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passengers: Option<ManifestPassengerLoad>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freight: Option<ManifestFreightLoad>,
    pub unavailable_fields: Vec<ManifestUnavailableField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightManifest {
    pub schema_version: u32,
    pub legs: Vec<FlightManifestLeg>,
}

impl FlightManifest {
    pub fn empty() -> Self {
        Self {
            schema_version: FLIGHT_MANIFEST_SCHEMA_VERSION,
            legs: Vec::new(),
        }
    }

    pub fn from_job(job: &JobSummary) -> Self {
        let legs = job
            .legs
            .iter()
            .map(|leg| {
                let passengers = leg.passengers.map(|count| ManifestPassengerLoad { count });
                let freight = leg
                    .cargo_weight_lb
                    .map(|weight_lb| ManifestFreightLoad { weight_lb });
                let mut unavailable_fields = Vec::new();
                match leg.kind {
                    JobLegKind::Passengers if passengers.is_none() => {
                        unavailable_fields.push(ManifestUnavailableField::PassengerCount);
                    }
                    JobLegKind::Cargo if freight.is_none() => {
                        unavailable_fields.push(ManifestUnavailableField::FreightWeight);
                    }
                    _ => {}
                }
                FlightManifestLeg {
                    source_job_leg_id: leg.id.clone(),
                    sequence: leg.sequence,
                    departure: leg.departure.clone(),
                    destination: leg.destination.clone(),
                    passengers,
                    freight,
                    unavailable_fields,
                }
            })
            .collect();
        Self {
            schema_version: FLIGHT_MANIFEST_SCHEMA_VERSION,
            legs,
        }
    }

    pub fn needs_attention(&self) -> bool {
        self.legs
            .iter()
            .any(|leg| !leg.unavailable_fields.is_empty())
    }

    pub fn validate(&self) -> Result<(), FlightOperationValidationError> {
        if self.schema_version != FLIGHT_MANIFEST_SCHEMA_VERSION
            || self.legs.len() > MAX_FLIGHT_MANIFEST_LEGS
        {
            return Err(FlightOperationValidationError::InvalidManifest);
        }
        let mut ids = HashSet::new();
        let mut previous_sequence = None;
        for leg in &self.legs {
            if !ids.insert(&leg.source_job_leg_id)
                || previous_sequence.is_some_and(|sequence| leg.sequence <= sequence)
                || leg
                    .freight
                    .as_ref()
                    .is_some_and(|load| !load.weight_lb.is_finite() || load.weight_lb < 0.0)
            {
                return Err(FlightOperationValidationError::InvalidManifest);
            }
            previous_sequence = Some(leg.sequence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightOperationRevision {
    pub schema_version: u32,
    pub operation_id: FlightOperationId,
    pub revision: u32,
    pub reason: FlightOperationRevisionReason,
    pub operation_created_at: DateTime<Utc>,
    pub revised_at: DateTime<Utc>,
    pub plan: FlightPlanSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_job_company_id: Option<CompanyId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_job: Option<OperationalObservation<JobSummary>>,
    pub manifest: FlightManifest,
}

impl FlightOperationRevision {
    pub fn validate(&self) -> Result<(), FlightOperationValidationError> {
        if self.schema_version != FLIGHT_OPERATION_SCHEMA_VERSION
            || self.operation_id.0.is_nil()
            || self.revision == 0
            || (self.revision == 1) != (self.reason == FlightOperationRevisionReason::Initial)
            || self.operation_created_at > self.revised_at
        {
            return Err(FlightOperationValidationError::InvalidRevision);
        }
        self.plan
            .validate()
            .map_err(|_| FlightOperationValidationError::InvalidPlan)?;
        match (&self.selected_job_company_id, &self.selected_job) {
            (Some(company_id), Some(job))
                if !company_id.0.is_nil()
                    && job.provenance.kind == ProvenanceKind::OnAirFact
                    && job.provenance.is_valid()
                    && job.value.validate().is_ok() => {}
            (None, None) => {}
            _ => return Err(FlightOperationValidationError::InvalidJob),
        }
        self.manifest.validate()?;
        match &self.selected_job {
            Some(job) if self.manifest != FlightManifest::from_job(&job.value) => {
                return Err(FlightOperationValidationError::InvalidManifest);
            }
            None if !self.manifest.legs.is_empty() => {
                return Err(FlightOperationValidationError::InvalidManifest);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FlightOperationValidationError {
    #[error("invalid flight-operation revision")]
    InvalidRevision,
    #[error("invalid flight-operation plan")]
    InvalidPlan,
    #[error("invalid flight-operation job")]
    InvalidJob,
    #[error("invalid flight-operation manifest")]
    InvalidManifest,
    #[error("invalid reviewed aircraft assignment")]
    InvalidAircraftAssignment,
}

#[cfg(test)]
#[path = "tests/flight_operation.rs"]
mod tests;

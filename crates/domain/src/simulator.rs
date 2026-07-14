use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Coordinates, OperationalProvenance, ProvenanceKind, valid_text};

pub const SIMULATOR_TELEMETRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatorIdentity {
    pub family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatorAircraftIdentity {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatorTelemetrySnapshot {
    pub schema_version: u32,
    pub sequence: u64,
    pub provenance: OperationalProvenance,
    pub simulator: SimulatorIdentity,
    pub aircraft: SimulatorAircraftIdentity,
    pub position: Coordinates,
    pub altitude_feet: f64,
    pub pitch_degrees: f64,
    pub bank_degrees: f64,
    pub true_heading_degrees: f64,
    pub indicated_airspeed_knots: f64,
    pub true_airspeed_knots: f64,
    pub ground_speed_knots: f64,
    pub on_ground: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_time_utc: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_total_gallons: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_total_weight_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_weight_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines_running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parking_brake_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_rate: Option<f64>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SimulatorTelemetryError {
    #[error("unsupported simulator telemetry schema version")]
    UnsupportedSchemaVersion,
    #[error("simulator telemetry sequence must be greater than zero")]
    InvalidSequence,
    #[error("simulator telemetry provenance must identify a current external fact")]
    InvalidProvenance,
    #[error("simulator or aircraft identity is invalid")]
    InvalidIdentity,
    #[error("simulator position is outside WGS84 bounds")]
    InvalidPosition,
    #[error("simulator telemetry contains an impossible or non-finite flight value")]
    InvalidFlightValue,
    #[error("simulator telemetry contains an impossible or non-finite quantity")]
    InvalidQuantity,
}

impl SimulatorTelemetrySnapshot {
    pub fn validate(&self) -> Result<(), SimulatorTelemetryError> {
        if self.schema_version != SIMULATOR_TELEMETRY_SCHEMA_VERSION {
            return Err(SimulatorTelemetryError::UnsupportedSchemaVersion);
        }
        if self.sequence == 0 {
            return Err(SimulatorTelemetryError::InvalidSequence);
        }
        if self.provenance.kind != ProvenanceKind::ExternalFact || !self.provenance.is_valid() {
            return Err(SimulatorTelemetryError::InvalidProvenance);
        }
        if !valid_text(&self.simulator.family, 64)
            || self
                .simulator
                .version
                .as_ref()
                .is_some_and(|version| !valid_text(version, 64))
            || !valid_text(&self.aircraft.title, 256)
            || self
                .aircraft
                .registration
                .as_ref()
                .is_some_and(|registration| !valid_text(registration, 32))
        {
            return Err(SimulatorTelemetryError::InvalidIdentity);
        }
        if !self.position.is_valid() {
            return Err(SimulatorTelemetryError::InvalidPosition);
        }

        let bounded_flight_values = [
            (self.altitude_feet, -2_000.0, 100_000.0),
            (self.pitch_degrees, -180.0, 180.0),
            (self.bank_degrees, -180.0, 180.0),
            (self.true_heading_degrees, 0.0, 360.0),
            (self.indicated_airspeed_knots, 0.0, 2_000.0),
            (self.true_airspeed_knots, 0.0, 2_000.0),
            (self.ground_speed_knots, 0.0, 2_000.0),
        ];
        if bounded_flight_values
            .into_iter()
            .any(|(value, minimum, maximum)| {
                !value.is_finite() || value < minimum || value > maximum
            })
        {
            return Err(SimulatorTelemetryError::InvalidFlightValue);
        }

        let non_negative_quantities = [
            self.fuel_total_gallons,
            self.fuel_total_weight_pounds,
            self.gross_weight_pounds,
        ];
        if non_negative_quantities
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || !(0.0..=10_000_000.0).contains(&value))
            || self
                .simulation_rate
                .is_some_and(|value| !value.is_finite() || !(0.0..=128.0).contains(&value))
        {
            return Err(SimulatorTelemetryError::InvalidQuantity);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/simulator.rs"]
mod tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{Coordinates, OperationalObservation, valid_code, valid_text};

pub const FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_FLIGHT_PLAN_ALTERNATES: usize = 8;
pub const MAX_FLIGHT_PLAN_ROUTE_LEGS: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlightPlanSnapshotId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightPlanSnapshot {
    pub schema_version: u32,
    pub id: FlightPlanSnapshotId,
    pub identity: OperationalObservation<FlightPlanIdentity>,
    pub airports: OperationalObservation<FlightPlanAirports>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft: Option<OperationalObservation<PlannedAircraft>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<OperationalObservation<PlannedSchedule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weights: Option<OperationalObservation<PlannedWeights>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel: Option<OperationalObservation<PlannedFuel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<OperationalObservation<PlannedRoute>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlightPlanIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_plan_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightPlanAirports {
    pub origin: FlightPlanAirport,
    pub destination: FlightPlanAirport,
    pub alternates: Vec<FlightPlanAirport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightPlanAirport {
    pub icao: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Coordinates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_runway: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedAircraft {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icao_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSchedule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_out: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_off: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_on: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_in: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_enroute_seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MassUnit {
    Kilograms,
    Pounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mass {
    pub value: f64,
    pub unit: MassUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedWeights {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zero_fuel: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub takeoff: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing: Option<Mass>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedFuel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxi: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enroute: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternate: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contingency: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ramp: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub takeoff: Option<Mass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing: Option<Mass>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_altitude_ft: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_nm: Option<f64>,
    pub legs: Vec<PlannedRouteLeg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedRouteLeg {
    pub sequence: u32,
    pub ident: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Coordinates>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FlightPlanValidationError {
    #[error("unsupported flight-plan snapshot schema version")]
    UnsupportedSchema,
    #[error("invalid flight-plan provenance")]
    InvalidProvenance,
    #[error("invalid flight-plan identity")]
    InvalidIdentity,
    #[error("invalid flight-plan airport")]
    InvalidAirport,
    #[error("too many flight-plan alternates")]
    TooManyAlternates,
    #[error("invalid planned aircraft")]
    InvalidAircraft,
    #[error("invalid planned schedule")]
    InvalidSchedule,
    #[error("invalid planned mass")]
    InvalidMass,
    #[error("invalid planned route")]
    InvalidRoute,
    #[error("too many planned route legs")]
    TooManyRouteLegs,
}

impl FlightPlanSnapshot {
    pub fn validate(&self) -> Result<(), FlightPlanValidationError> {
        if self.schema_version != FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION {
            return Err(FlightPlanValidationError::UnsupportedSchema);
        }

        let mut provenance = vec![&self.identity.provenance, &self.airports.provenance];
        provenance.extend(self.aircraft.iter().map(|group| &group.provenance));
        provenance.extend(self.schedule.iter().map(|group| &group.provenance));
        provenance.extend(self.weights.iter().map(|group| &group.provenance));
        provenance.extend(self.fuel.iter().map(|group| &group.provenance));
        provenance.extend(self.route.iter().map(|group| &group.provenance));
        if provenance.iter().any(|item| !item.is_valid()) {
            return Err(FlightPlanValidationError::InvalidProvenance);
        }

        if self
            .identity
            .value
            .airac
            .as_ref()
            .is_some_and(|value| !valid_text(value, 16))
            || self
                .identity
                .value
                .provider_plan_reference
                .as_ref()
                .is_some_and(|value| !valid_text(value, 128))
        {
            return Err(FlightPlanValidationError::InvalidIdentity);
        }

        if !valid_airport(&self.airports.value.origin)
            || !valid_airport(&self.airports.value.destination)
            || self
                .airports
                .value
                .alternates
                .iter()
                .any(|airport| !valid_airport(airport))
        {
            return Err(FlightPlanValidationError::InvalidAirport);
        }
        if self.airports.value.alternates.len() > MAX_FLIGHT_PLAN_ALTERNATES {
            return Err(FlightPlanValidationError::TooManyAlternates);
        }

        if self.aircraft.as_ref().is_some_and(|group| {
            let aircraft = &group.value;
            aircraft
                .icao_type
                .as_ref()
                .is_some_and(|value| !valid_code(value, 2, 16))
                || aircraft
                    .registration
                    .as_ref()
                    .is_some_and(|value| !valid_text(value, 32))
                || aircraft
                    .model
                    .as_ref()
                    .is_some_and(|value| !valid_text(value, 160))
        }) {
            return Err(FlightPlanValidationError::InvalidAircraft);
        }

        if self.schedule.as_ref().is_some_and(|group| {
            let schedule = &group.value;
            schedule
                .estimated_enroute_seconds
                .is_some_and(|seconds| seconds > 7 * 24 * 60 * 60)
                || schedule
                    .scheduled_out
                    .zip(schedule.scheduled_in)
                    .is_some_and(|(out, r#in)| out > r#in)
        }) {
            return Err(FlightPlanValidationError::InvalidSchedule);
        }

        if self.weights.as_ref().is_some_and(|group| {
            weights(&group.value)
                .into_iter()
                .flatten()
                .any(invalid_mass)
        }) || self
            .fuel
            .as_ref()
            .is_some_and(|group| fuel(&group.value).into_iter().flatten().any(invalid_mass))
        {
            return Err(FlightPlanValidationError::InvalidMass);
        }

        if let Some(route) = &self.route {
            if route.value.legs.len() > MAX_FLIGHT_PLAN_ROUTE_LEGS {
                return Err(FlightPlanValidationError::TooManyRouteLegs);
            }
            if route
                .value
                .source_text
                .as_ref()
                .is_some_and(|value| !valid_text(value, 32 * 1024))
                || route
                    .value
                    .initial_altitude_ft
                    .is_some_and(|altitude| altitude > 70_000)
                || route.value.distance_nm.is_some_and(|distance| {
                    !distance.is_finite() || !(0.0..=30_000.0).contains(&distance)
                })
                || route.value.legs.iter().enumerate().any(|(index, leg)| {
                    leg.sequence != index as u32
                        || !valid_code(&leg.ident, 1, 24)
                        || leg
                            .airway
                            .as_ref()
                            .is_some_and(|value| !valid_code(value, 1, 24))
                        || leg.location.is_some_and(|location| !location.is_valid())
                })
            {
                return Err(FlightPlanValidationError::InvalidRoute);
            }
        }

        Ok(())
    }
}

fn valid_airport(value: &FlightPlanAirport) -> bool {
    valid_code(&value.icao, 2, 8)
        && value.name.as_ref().is_none_or(|name| valid_text(name, 160))
        && value.location.is_none_or(Coordinates::is_valid)
        && value
            .planned_runway
            .as_ref()
            .is_none_or(|runway| valid_code(runway, 1, 12))
}

fn invalid_mass(value: Mass) -> bool {
    !value.value.is_finite() || !(0.0..=2_000_000_000.0).contains(&value.value)
}

fn weights(value: &PlannedWeights) -> [Option<Mass>; 4] {
    [value.payload, value.zero_fuel, value.takeoff, value.landing]
}

fn fuel(value: &PlannedFuel) -> [Option<Mass>; 9] {
    [
        value.taxi,
        value.enroute,
        value.reserve,
        value.alternate,
        value.contingency,
        value.extra,
        value.ramp,
        value.takeoff,
        value.landing,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperationalProvenance, ProvenanceKind, SnapshotFreshness};

    fn provenance() -> OperationalProvenance {
        OperationalProvenance {
            kind: ProvenanceKind::ExternalCalculation,
            provider: "simbrief".into(),
            provider_revision: Some("AIRAC 2607".into()),
            generated_at: DateTime::from_timestamp(1_782_864_000, 0),
            retrieved_at: DateTime::from_timestamp(1_782_867_600, 0).unwrap(),
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        }
    }

    fn snapshot() -> FlightPlanSnapshot {
        FlightPlanSnapshot {
            schema_version: FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION,
            id: FlightPlanSnapshotId(Uuid::nil()),
            identity: OperationalObservation {
                value: FlightPlanIdentity {
                    airac: Some("2607".into()),
                    provider_plan_reference: None,
                },
                provenance: provenance(),
            },
            airports: OperationalObservation {
                value: FlightPlanAirports {
                    origin: FlightPlanAirport {
                        icao: "YSSY".into(),
                        name: Some("Sydney".into()),
                        location: Some(Coordinates {
                            latitude: -33.9461,
                            longitude: 151.1772,
                        }),
                        planned_runway: Some("34L".into()),
                    },
                    destination: FlightPlanAirport {
                        icao: "YMML".into(),
                        name: Some("Melbourne".into()),
                        location: None,
                        planned_runway: None,
                    },
                    alternates: Vec::new(),
                },
                provenance: provenance(),
            },
            aircraft: None,
            schedule: None,
            weights: None,
            fuel: None,
            route: Some(OperationalObservation {
                value: PlannedRoute {
                    source_text: Some("DCT TESAT Q29 LIZZI".into()),
                    initial_altitude_ft: Some(35_000),
                    distance_nm: Some(389.0),
                    legs: vec![PlannedRouteLeg {
                        sequence: 0,
                        ident: "TESAT".into(),
                        airway: Some("Q29".into()),
                        location: None,
                    }],
                },
                provenance: provenance(),
            }),
        }
    }

    #[test]
    fn validates_a_bounded_provider_neutral_snapshot() {
        assert_eq!(snapshot().validate(), Ok(()));
    }

    #[test]
    fn rejects_non_finite_mass_and_out_of_sequence_route_legs() {
        let mut candidate = snapshot();
        candidate.weights = Some(OperationalObservation {
            value: PlannedWeights {
                payload: Some(Mass {
                    value: f64::NAN,
                    unit: MassUnit::Kilograms,
                }),
                zero_fuel: None,
                takeoff: None,
                landing: None,
            },
            provenance: provenance(),
        });
        assert_eq!(
            candidate.validate(),
            Err(FlightPlanValidationError::InvalidMass)
        );

        let mut candidate = snapshot();
        candidate.route.as_mut().unwrap().value.legs[0].sequence = 7;
        assert_eq!(
            candidate.validate(),
            Err(FlightPlanValidationError::InvalidRoute)
        );
    }

    #[test]
    fn validates_the_version_one_json_fixture() {
        let snapshot: FlightPlanSnapshot = serde_json::from_str(include_str!(
            "../../../schemas/fixtures/flight-plan-snapshot-v1.json"
        ))
        .unwrap();
        assert_eq!(snapshot.validate(), Ok(()));
    }
}

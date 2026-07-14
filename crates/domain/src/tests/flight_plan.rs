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
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap();
    assert_eq!(snapshot.validate(), Ok(()));
}

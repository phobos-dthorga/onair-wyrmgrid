use super::*;
use crate::{SIMULATOR_TELEMETRY_SCHEMA_VERSION, SnapshotFreshness};

fn snapshot() -> SimulatorTelemetrySnapshot {
    SimulatorTelemetrySnapshot {
        schema_version: SIMULATOR_TELEMETRY_SCHEMA_VERSION,
        sequence: 1,
        provenance: OperationalProvenance {
            kind: ProvenanceKind::ExternalFact,
            provider: "msfs2024-simconnect".into(),
            provider_revision: Some("1.6.9".into()),
            generated_at: None,
            retrieved_at: "2026-07-15T04:00:00Z"
                .parse()
                .expect("timestamp should parse"),
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        },
        simulator: SimulatorIdentity {
            family: "msfs_2024".into(),
            version: Some("1.6.9".into()),
        },
        aircraft: SimulatorAircraftIdentity {
            title: "Sanitized test aircraft".into(),
            registration: Some("VH-WRM".into()),
        },
        position: Coordinates {
            latitude: -33.8688,
            longitude: 151.2093,
        },
        altitude_feet: 4_500.0,
        pitch_degrees: 2.0,
        bank_degrees: -1.0,
        true_heading_degrees: 271.0,
        indicated_airspeed_knots: 128.0,
        true_airspeed_knots: 136.0,
        ground_speed_knots: 132.0,
        on_ground: false,
        simulation_time_utc: Some(
            "2026-07-15T04:00:00Z"
                .parse()
                .expect("timestamp should parse"),
        ),
        fuel_total_gallons: Some(81.0),
        fuel_total_weight_pounds: Some(486.0),
        gross_weight_pounds: Some(4_820.0),
        engines_running: Some(true),
        parking_brake_set: Some(false),
        paused: Some(false),
        simulation_rate: Some(1.0),
    }
}

#[test]
fn accepts_a_bounded_external_simulator_snapshot() {
    assert_eq!(snapshot().validate(), Ok(()));
}

#[test]
fn rejects_non_finite_and_out_of_bounds_flight_values() {
    let mut candidate = snapshot();
    candidate.position.latitude = 91.0;
    assert_eq!(
        candidate.validate(),
        Err(SimulatorTelemetryError::InvalidPosition)
    );

    candidate = snapshot();
    candidate.ground_speed_knots = f64::NAN;
    assert_eq!(
        candidate.validate(),
        Err(SimulatorTelemetryError::InvalidFlightValue)
    );
}

#[test]
fn rejects_non_external_provenance_and_invalid_quantities() {
    let mut candidate = snapshot();
    candidate.provenance.kind = ProvenanceKind::Calculated;
    assert_eq!(
        candidate.validate(),
        Err(SimulatorTelemetryError::InvalidProvenance)
    );

    candidate = snapshot();
    candidate.fuel_total_gallons = Some(-1.0);
    assert_eq!(
        candidate.validate(),
        Err(SimulatorTelemetryError::InvalidQuantity)
    );
}

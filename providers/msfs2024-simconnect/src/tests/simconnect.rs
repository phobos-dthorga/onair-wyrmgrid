use super::*;

fn text<const N: usize>(value: &str) -> [c_char; N] {
    let mut output = [0; N];
    for (target, source) in output.iter_mut().zip(value.bytes()) {
        *target = source as c_char;
    }
    output
}

fn raw() -> RawTelemetry {
    RawTelemetry {
        title: text("Sanitized test aircraft"),
        registration: text("VH-WRM"),
        latitude: -33.8688,
        longitude: 151.2093,
        altitude_feet: 4_500.0,
        pitch_degrees: 2.0,
        bank_degrees: -1.0,
        true_heading_degrees: 271.0,
        indicated_airspeed_knots: 128.0,
        true_airspeed_knots: 136.0,
        ground_speed_knots: 132.0,
        on_ground: 0.0,
        zulu_year: 2026.0,
        zulu_month: 7.0,
        zulu_day: 15.0,
        zulu_seconds: 14_400.0,
        fuel_total_gallons: 81.0,
        fuel_total_weight_pounds: 486.0,
        gross_weight_pounds: 4_820.0,
        engine_1_combustion: 1.0,
        engine_2_combustion: 0.0,
        engine_3_combustion: 0.0,
        engine_4_combustion: 0.0,
        parking_brake: 0.0,
    }
}

#[test]
fn translates_raw_simconnect_values_into_the_stable_snapshot() {
    let snapshot = raw()
        .into_snapshot(1, Some(false), Some("1.6.9".into()), Some("1.6.9".into()))
        .expect("raw values should translate");

    assert_eq!(snapshot.aircraft.title, "Sanitized test aircraft");
    assert_eq!(snapshot.aircraft.registration.as_deref(), Some("VH-WRM"));
    assert_eq!(snapshot.position.latitude, -33.8688);
    assert_eq!(
        snapshot.simulation_time_utc.unwrap().to_rfc3339(),
        "2026-07-15T04:00:00+00:00"
    );
    assert_eq!(snapshot.engines_running, Some(true));
    assert_eq!(snapshot.provenance.kind, ProvenanceKind::ExternalFact);
}

#[test]
fn rejects_impossible_raw_coordinates_before_publication() {
    let mut candidate = raw();
    candidate.latitude = 200.0;
    assert!(matches!(
        candidate.into_snapshot(1, None, None, None),
        Err(SimConnectError::InvalidTelemetry)
    ));
}

use super::*;
use crate::{SimulatorCaptureMode, SimulatorRecordingStatus, SimulatorSessionSummary};

fn session(sample_count: u64) -> SimulatorSessionSummary {
    SimulatorSessionSummary {
        id: "session-1".into(),
        provider_id: "provider-1".into(),
        simulator_family: "MSFS_2024".into(),
        simulator_version: None,
        aircraft_title: "Test aircraft".into(),
        aircraft_registration: None,
        started_at: "2026-07-16T00:00:00Z".into(),
        ended_at: None,
        status: SimulatorRecordingStatus::Completed,
        sample_count,
        capture_mode: SimulatorCaptureMode::Manual,
        pinned: false,
        plan_associated: false,
    }
}

fn sample(index: usize) -> SimulatorRecordedSample {
    SimulatorRecordedSample {
        source_sequence: index as u64,
        observed_at: format!("2026-07-16T00:{:02}:{:02}Z", (index / 60) % 60, index % 60),
        simulation_time_utc: None,
        altitude_feet: index as f64,
        indicated_airspeed_knots: (index % 200) as f64,
        true_airspeed_knots: (index % 220) as f64,
        ground_speed_knots: (index % 180) as f64,
        fuel_total_weight_pounds: Some(10_000.0 - index as f64),
        gross_weight_pounds: None,
        pitch_degrees: (index % 20) as f64 - 10.0,
        bank_degrees: (index % 40) as f64 - 20.0,
        gap_before: false,
        position: Some(Coordinates {
            latitude: -33.0 + index as f64 / 10_000.0,
            longitude: 151.0 + index as f64 / 10_000.0,
        }),
        on_ground: None,
        engines_running: None,
        parking_brake_set: None,
        paused: None,
    }
}

#[test]
fn short_debriefs_retain_exact_samples() {
    let samples = (0..12).map(sample).collect::<Vec<_>>();
    let debrief = build_simulator_debrief(session(12), &samples, None, None);
    assert_eq!(
        debrief.traces.altitude.method,
        SimulatorDownsamplingMethod::Exact
    );
    assert_eq!(debrief.traces.altitude.samples, samples);
    assert_eq!(debrief.route.recorded.points.len(), 12);
}

#[test]
fn long_debriefs_are_bounded_and_preserve_extrema_and_gaps() {
    let mut samples = (0..5_000).map(sample).collect::<Vec<_>>();
    samples[2_500].altitude_feet = 60_000.0;
    samples[3_000].gap_before = true;
    let debrief = build_simulator_debrief(session(5_000), &samples, None, None);
    let altitude = &debrief.traces.altitude;
    assert_eq!(altitude.method, SimulatorDownsamplingMethod::MinMaxEnvelope);
    assert!(altitude.samples.len() <= MAX_DEBRIEF_TRACE_POINTS);
    assert_eq!(altitude.samples.first().unwrap().source_sequence, 0);
    assert_eq!(altitude.samples.last().unwrap().source_sequence, 4_999);
    assert!(
        altitude
            .samples
            .iter()
            .any(|sample| sample.altitude_feet == 60_000.0)
    );
    assert!(altitude.samples.iter().any(|sample| sample.gap_before));
    assert_eq!(altitude.gap_count, 1);
}

#[test]
fn unavailable_fuel_does_not_create_a_trace() {
    let mut samples = (0..10).map(sample).collect::<Vec<_>>();
    for sample in &mut samples {
        sample.fuel_total_weight_pounds = None;
    }
    let debrief = build_simulator_debrief(session(10), &samples, None, None);
    assert!(debrief.traces.fuel.is_none());
}

#[test]
fn missing_positions_break_recorded_geometry_instead_of_being_joined() {
    let mut samples = (0..4).map(sample).collect::<Vec<_>>();
    samples[1].position = None;
    let debrief = build_simulator_debrief(session(4), &samples, None, None);
    assert_eq!(debrief.route.recorded.points.len(), 3);
    assert!(debrief.route.recorded.points[1].gap_before);
}

#[test]
fn unresolved_plan_legs_remain_unplotted_and_break_the_plan_line() {
    let mut plan: FlightPlanSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap();
    plan.airports.value.origin.location = Some(Coordinates {
        latitude: -33.946_111,
        longitude: 151.177_222,
    });
    plan.airports.value.destination.location = Some(Coordinates {
        latitude: -37.008_056,
        longitude: 174.791_667,
    });
    let legs = &mut plan.route.as_mut().unwrap().value.legs;
    legs[0].location = Some(Coordinates {
        latitude: -34.2,
        longitude: 153.0,
    });
    legs[2].location = Some(Coordinates {
        latitude: -36.0,
        longitude: 170.0,
    });
    let expected_unresolved = plan
        .route
        .as_ref()
        .unwrap()
        .value
        .legs
        .iter()
        .filter(|leg| leg.location.is_none())
        .count();
    let samples = (0..4).map(sample).collect::<Vec<_>>();
    let debrief = build_simulator_debrief(session(4), &samples, Some(&plan), None);
    let planned = debrief.route.planned.unwrap();
    assert_eq!(planned.unresolved_legs.len(), expected_unresolved);
    assert!(planned.points.iter().skip(1).any(|point| point.gap_before));
}

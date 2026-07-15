use super::*;
use wyrmgrid_bridge_protocol::{BridgeEnvelope, BridgeProviderMessage};

fn snapshot() -> SimulatorTelemetrySnapshot {
    let envelope: BridgeEnvelope<BridgeProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/simulator-telemetry-v1.json"
    ))
    .expect("telemetry fixture should deserialize");
    let BridgeProviderMessage::Telemetry { snapshot } = envelope.payload else {
        panic!("fixture should contain telemetry");
    };
    snapshot
}

fn automatic_preferences() -> SimulatorRecordingPreferences {
    SimulatorRecordingPreferences {
        retention_days: 30,
        automatic_start: true,
        automatic_stop: true,
        landing_settle_seconds: 10,
    }
}

#[test]
fn manual_recording_requires_fresh_telemetry() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    assert!(matches!(
        service.start("provider-1", None),
        Err(SimulatorRecordingError::FreshTelemetryRequired)
    ));
}

#[test]
fn manual_recording_persists_samples_and_marks_stream_gaps() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let first = snapshot();
    let started = service
        .start("provider-1", Some(first.clone()))
        .expect("recording should start");
    let session_id = started.active_session_id.unwrap();

    let mut second = first.clone();
    second.sequence += 1;
    second.provenance.retrieved_at += Duration::seconds(1);
    service.observe_snapshot("provider-1", &second);

    let mut after_gap = second;
    after_gap.sequence += 2;
    after_gap.provenance.retrieved_at += Duration::seconds(4);
    service.observe_snapshot("provider-1", &after_gap);

    let detail = service.session(&session_id).unwrap();
    assert_eq!(detail.samples.len(), 3);
    assert!(!detail.samples[0].gap_before);
    assert!(!detail.samples[1].gap_before);
    assert!(detail.samples[2].gap_before);

    let stopped = service.stop().unwrap();
    assert!(stopped.active_session_id.is_none());
    assert_eq!(
        stopped.sessions[0].status,
        SimulatorRecordingStatus::Completed
    );
}

#[test]
fn provider_clock_regressions_are_recorded_as_stream_gaps() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let first = snapshot();
    let started = service
        .start("provider-1", Some(first.clone()))
        .expect("recording should start");
    let session_id = started.active_session_id.unwrap();

    let mut regressed = first;
    regressed.sequence += 1;
    regressed.provenance.retrieved_at -= Duration::seconds(1);
    service.observe_snapshot("provider-1", &regressed);

    let detail = service.session(&session_id).unwrap();
    assert_eq!(detail.samples.len(), 2);
    assert!(!detail.samples[0].gap_before);
    assert!(detail.samples[1].gap_before);
    assert_eq!(
        detail
            .events
            .iter()
            .filter(|event| event.event_kind == "telemetry_gap")
            .count(),
        1
    );
}

#[test]
fn aircraft_changes_interrupt_instead_of_joining_unrelated_samples() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let first = snapshot();
    service.start("provider-1", Some(first.clone())).unwrap();

    let mut changed = first;
    changed.sequence += 1;
    changed.aircraft.title = "Different aircraft".into();
    service.observe_snapshot("provider-1", &changed);

    let status = service.status().unwrap();
    assert!(status.active_session_id.is_none());
    assert_eq!(
        status.sessions[0].status,
        SimulatorRecordingStatus::Interrupted
    );
    assert_eq!(
        status.last_code.as_deref(),
        Some("recording.aircraft_changed")
    );
}

#[test]
fn retention_is_bounded_and_deletion_rejects_the_active_session() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    assert!(matches!(
        service.update_preferences(SimulatorRecordingPreferences {
            retention_days: 0,
            ..automatic_preferences()
        }),
        Err(SimulatorRecordingError::InvalidRetention)
    ));
    let status = service.start("provider-1", Some(snapshot())).unwrap();
    let session_id = status.active_session_id.unwrap();
    assert!(matches!(
        service.delete_session(&session_id),
        Err(SimulatorRecordingError::ActiveSession)
    ));
    service.stop().unwrap();
    assert!(
        service
            .delete_session(&session_id)
            .unwrap()
            .sessions
            .is_empty()
    );
}

#[test]
fn automatic_recording_is_opt_in_and_records_reviewable_lifecycle_evidence() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let first = snapshot();
    service.observe_snapshot("provider-1", &first);
    assert!(service.status().unwrap().active_session_id.is_none());

    service.update_preferences(automatic_preferences()).unwrap();
    service.observe_snapshot("provider-1", &first);
    let mut confirmed_airborne = first.clone();
    confirmed_airborne.sequence += 1;
    confirmed_airborne.provenance.retrieved_at += Duration::seconds(1);
    service.observe_snapshot("provider-1", &confirmed_airborne);

    let active = service.status().unwrap();
    let session_id = active
        .active_session_id
        .expect("two airborne observations should start an automatic recording");
    assert_eq!(
        active.sessions[0].capture_mode,
        SimulatorCaptureMode::Automatic
    );

    let mut landed = confirmed_airborne.clone();
    landed.sequence += 1;
    landed.on_ground = true;
    landed.ground_speed_knots = 15.0;
    landed.provenance.retrieved_at += Duration::seconds(1);
    service.observe_snapshot("provider-1", &landed);
    assert!(service.status().unwrap().active_session_id.is_some());

    let mut settled = landed;
    for _ in 0..10 {
        settled.sequence += 1;
        settled.ground_speed_knots = 0.0;
        settled.provenance.retrieved_at += Duration::seconds(1);
        service.observe_snapshot("provider-1", &settled);
    }

    let completed = service.status().unwrap();
    assert!(completed.active_session_id.is_none());
    assert_eq!(
        completed.sessions[0].status,
        SimulatorRecordingStatus::Completed
    );
    let detail = service.session(&session_id).unwrap();
    assert_eq!(
        detail
            .events
            .iter()
            .map(|event| event.event_kind.as_str())
            .collect::<Vec<_>>(),
        vec!["takeoff_confirmed", "landing_settled"]
    );
}

#[test]
fn automatic_lifecycle_requires_continuous_unpaused_evidence() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    service.update_preferences(automatic_preferences()).unwrap();
    let first = snapshot();
    service.observe_snapshot("provider-1", &first);

    let mut after_gap = first.clone();
    after_gap.sequence += 2;
    after_gap.provenance.retrieved_at += Duration::seconds(GAP_AFTER_SECONDS + 1);
    service.observe_snapshot("provider-1", &after_gap);
    assert!(service.status().unwrap().active_session_id.is_none());

    let mut confirmed = after_gap.clone();
    confirmed.sequence += 1;
    confirmed.provenance.retrieved_at += Duration::seconds(1);
    service.observe_snapshot("provider-1", &confirmed);
    assert!(service.status().unwrap().active_session_id.is_some());

    let mut paused_on_ground = confirmed;
    paused_on_ground.on_ground = true;
    paused_on_ground.simulation_rate = Some(0.0);
    for _ in 0..15 {
        paused_on_ground.sequence += 1;
        paused_on_ground.provenance.retrieved_at += Duration::seconds(1);
        service.observe_snapshot("provider-1", &paused_on_ground);
    }
    assert!(service.status().unwrap().active_session_id.is_some());
}

#[test]
fn simbrief_plan_is_snapshotted_and_compared_only_with_available_recorded_facts() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let plan: FlightPlanSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap();
    service.set_plan_context(Some(plan)).unwrap();
    let first = snapshot();
    let started = service.start("provider-1", Some(first.clone())).unwrap();
    let session_id = started.active_session_id.unwrap();
    let mut second = first;
    second.sequence += 1;
    second.altitude_feet = 5_500.0;
    second.provenance.retrieved_at += Duration::seconds(5);
    service.observe_snapshot("provider-1", &second);

    let detail = service.session(&session_id).unwrap();
    assert!(detail.session.plan_associated);
    let comparison = detail.comparison.expect("plan should be correlated");
    assert_eq!(comparison.association.origin_icao, "YSSY");
    assert_eq!(comparison.association.destination_icao, "NZAA");
    assert_eq!(comparison.association.correlation_version, 2);
    assert_eq!(comparison.planned_distance_nm, Some(1_184.6));
    assert_eq!(comparison.recorded_peak_altitude_ft, Some(5_500.0));
    assert_eq!(comparison.recorded_seconds, Some(5));
    assert_eq!(comparison.planned_enroute_seconds, None);
    assert_eq!(comparison.altitude_delta_ft, Some(-30_500.0));

    let debrief = service.debrief(&session_id).unwrap();
    assert_eq!(debrief.schema_version, 1);
    assert_eq!(debrief.source_sample_count, 2);
    assert_eq!(
        debrief
            .route
            .planned
            .as_ref()
            .expect("stored plan should project")
            .unresolved_legs,
        ["TESAT", "LIZZI", "LUNBI"]
    );

    let json = service
        .export_session(&session_id, SimulatorExportFormat::Json)
        .unwrap();
    assert_eq!(json.media_type, "application/json");
    let exported: serde_json::Value = serde_json::from_str(&json.content).unwrap();
    assert_eq!(exported["schema_version"], 1);
    assert_eq!(
        exported["plan_snapshot"]["airports"]["value"]["origin"]["icao"],
        "YSSY"
    );
    let csv = service
        .export_session(&session_id, SimulatorExportFormat::Csv)
        .unwrap();
    assert!(csv.content.starts_with("source_sequence,observed_at"));
}

#[test]
fn recording_windows_can_move_back_through_exact_samples() {
    let service = SimulatorRecordingService::new(Store::open_in_memory().unwrap());
    let first = snapshot();
    let started = service.start("provider-1", Some(first.clone())).unwrap();
    let session_id = started.active_session_id.unwrap();
    for offset in 1..=605 {
        let mut sample = first.clone();
        sample.sequence += offset;
        sample.provenance.retrieved_at += Duration::seconds(offset as i64);
        service.observe_snapshot("provider-1", &sample);
    }

    let newest = service.session_window(&session_id, 0).unwrap();
    assert_eq!(newest.samples.len(), 600);
    assert!(newest.has_older_samples);
    assert!(!newest.has_newer_samples);
    let older = service.session_window(&session_id, 600).unwrap();
    assert_eq!(older.samples.len(), 6);
    assert!(!older.has_older_samples);
    assert!(older.has_newer_samples);
}

#[test]
fn fuel_use_is_unavailable_when_the_recorded_quantity_increases() {
    let first = snapshot();
    let mut second = first.clone();
    second.sequence += 1;
    second.fuel_total_weight_pounds = first.fuel_total_weight_pounds.map(|fuel| fuel + 5.0);
    let samples = vec![
        sample_from_record(sample_from_snapshot(&first, false)),
        sample_from_record(sample_from_snapshot(&second, false)),
    ];
    assert_eq!(fuel_used_pounds(&samples), None);
}

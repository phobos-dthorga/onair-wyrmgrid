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
        service.update_preferences(SimulatorRecordingPreferences { retention_days: 0 }),
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

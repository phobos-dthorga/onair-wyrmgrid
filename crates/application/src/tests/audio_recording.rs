use super::*;
use crate::{AudioProviderCaptureBatch, AudioProviderEvent, AudioProviderPacket};
use std::sync::atomic::{AtomicBool, Ordering};
use wyrmgrid_audio_provider_protocol::{AudioCaptureEventKind, AudioStartedTrack, AudioStopReason};
use wyrmgrid_domain::{
    AUDIO_SOURCE_SCHEMA_VERSION, AudioPermissionState, AudioSourceDirection, AudioSourceOrigin,
    AudioSourceTruth,
};

struct FakeProvider {
    permission_granted: AtomicBool,
}

impl FakeProvider {
    fn new() -> Self {
        Self {
            permission_granted: AtomicBool::new(false),
        }
    }

    fn source(&self) -> AudioSourceCapability {
        AudioSourceCapability {
            schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
            id: "synthetic.microphone.primary".into(),
            display_name: "Synthetic microphone".into(),
            role: AudioSourceRole::MicrophoneInput,
            direction: AudioSourceDirection::Input,
            truth: AudioSourceTruth::Isolated,
            availability: AudioSourceAvailability::Available,
            permission: if self.permission_granted.load(Ordering::SeqCst) {
                AudioPermissionState::Granted
            } else {
                AudioPermissionState::PromptRequired
            },
            channels: 1,
            native_sample_rate_hz: 48_000,
            supported_profiles: vec![AudioOpusProfileId::PilotMicrophoneV1],
            supports_hot_plug: true,
            origin: AudioSourceOrigin::OperatingSystem,
        }
    }
}

impl AudioCaptureProvider for FakeProvider {
    fn provider_id(&self) -> &str {
        "dev.wyrmgrid.fake-audio"
    }

    fn sources(&self) -> Result<Vec<AudioSourceCapability>, AudioProviderError> {
        Ok(vec![self.source()])
    }

    fn request_permission(
        &self,
        source_id: &str,
    ) -> Result<Vec<AudioSourceCapability>, AudioProviderError> {
        if source_id != "synthetic.microphone.primary" {
            return Err(AudioProviderError::SourceUnavailable);
        }
        self.permission_granted.store(true, Ordering::SeqCst);
        Ok(vec![self.source()])
    }

    fn start_capture(
        &self,
        session_id: &str,
        tracks: &[AudioTrackRequest],
    ) -> Result<AudioProviderCaptureBatch, AudioProviderError> {
        let track = tracks.first().ok_or(AudioProviderError::Protocol)?;
        Ok(AudioProviderCaptureBatch {
            provider_start_monotonic_ns: 1_010_000,
            tracks: vec![AudioStartedTrack {
                track_id: track.track_id.clone(),
                source_id: track.source_id.clone(),
                profile: track.profile,
                provider_start_monotonic_ns: 1_010_000,
            }],
            packets: vec![AudioProviderPacket {
                session_id: session_id.into(),
                track_id: track.track_id.clone(),
                packet: EncodedAudioPacket {
                    sequence: 1,
                    provider_monotonic_ns: 1_020_000,
                    duration_48khz_frames: 960,
                    bytes: vec![0xf8, 0xff, 0xfe, 0x00],
                },
            }],
            levels: vec![AudioProviderLevel {
                session_id: session_id.into(),
                track_id: track.track_id.clone(),
                provider_monotonic_ns: 1_020_000,
                peak_millidbfs: -12_000,
                clipped: false,
            }],
            events: vec![AudioProviderEvent {
                session_id: session_id.into(),
                track_id: Some(track.track_id.clone()),
                provider_monotonic_ns: 1_040_000,
                event: AudioCaptureEventKind::Gap,
                code: "capture.synthetic_gap".into(),
                affected_frames: Some(960),
                drift_parts_per_million: None,
            }],
        })
    }

    fn stop_capture(&self, _: &str) -> Result<AudioStopReason, AudioProviderError> {
        Ok(AudioStopReason::UserRequested)
    }
}

fn service() -> (AudioRecordingService, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let media = EncryptedAudioMediaStore::new(
        directory.path(),
        crate::AudioMediaKey::from_test_bytes([11; 32]),
    );
    (
        AudioRecordingService::new(
            Store::open_in_memory().unwrap(),
            media,
            Some(Arc::new(FakeProvider::new())),
        ),
        directory,
    )
}

fn enabled_preferences() -> AudioRecordingPreferences {
    AudioRecordingPreferences {
        enabled: true,
        capture_manual: true,
        capture_automatic: false,
        ..AudioRecordingPreferences::default()
    }
}

fn selection() -> AudioSourceSelection {
    AudioSourceSelection {
        provider_id: "dev.wyrmgrid.fake-audio".into(),
        source_id: "synthetic.microphone.primary".into(),
        profile_id: AudioOpusProfileId::PilotMicrophoneV1,
        enabled: true,
        playback_muted: false,
        playback_solo: false,
        playback_volume_percent: 100,
    }
}

#[test]
fn audio_consent_and_capture_modes_default_off() {
    let (service, _directory) = service();
    let status = service.status().unwrap();
    assert!(!status.preferences.enabled);
    assert!(!status.preferences.capture_manual);
    assert!(!status.preferences.capture_automatic);
    assert_eq!(
        service.start(None, AudioCaptureMode::Manual).unwrap_err(),
        AudioRecordingError::ConsentDisabled
    );
}

#[test]
fn permission_is_never_requested_implicitly() {
    let (service, _directory) = service();
    service.update_preferences(enabled_preferences()).unwrap();
    service.refresh_sources().unwrap();
    service.update_source_selection(selection()).unwrap();
    assert_eq!(
        service.start(None, AudioCaptureMode::Manual).unwrap_err(),
        AudioRecordingError::PermissionRequired
    );
    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    assert!(
        service
            .start(None, AudioCaptureMode::Manual)
            .unwrap()
            .recording_active
    );
}

#[test]
fn capture_persists_authenticated_playback_export_and_coordinated_deletion() {
    let (service, directory) = service();
    service.update_preferences(enabled_preferences()).unwrap();
    service.refresh_sources().unwrap();
    service.update_source_selection(selection()).unwrap();
    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    let active = service.start(None, AudioCaptureMode::Manual).unwrap();
    let session_id = active.active_session_id.unwrap();
    assert_eq!(
        service.playback(&session_id).unwrap_err(),
        AudioRecordingError::SessionActive
    );
    assert_eq!(
        service.delete_session(&session_id).unwrap_err(),
        AudioRecordingError::SessionActive
    );
    let completed = service.stop().unwrap();
    assert!(!completed.recording_active);

    let playback = service.playback(&session_id).unwrap();
    assert!(playback.authenticated);
    assert_eq!(playback.tracks.len(), 1);
    assert_eq!(
        playback.tracks[0].packets[0].bytes,
        vec![0xf8, 0xff, 0xfe, 0x00]
    );

    let export_path = directory.path().join("track.wyrmgrid-opus-packets");
    let export = service
        .export_track(&session_id, &playback.tracks[0].track_id, &export_path)
        .unwrap();
    assert!(export.plaintext_warning_required);
    assert_eq!(export.packet_count, 1);
    assert!(export_path.is_file());

    assert!(
        service
            .delete_session(&session_id)
            .unwrap()
            .sessions
            .is_empty()
    );
}

#[test]
fn automatic_capture_needs_its_own_explicit_consent() {
    let (service, _directory) = service();
    service.update_preferences(enabled_preferences()).unwrap();
    service.refresh_sources().unwrap();
    service.update_source_selection(selection()).unwrap();
    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    assert_eq!(
        service
            .start(None, AudioCaptureMode::Automatic)
            .unwrap_err(),
        AudioRecordingError::CaptureModeDisabled
    );
}

#[test]
fn revoking_master_or_source_consent_stops_active_capture() {
    let (service, _directory) = service();
    service.update_preferences(enabled_preferences()).unwrap();
    service.refresh_sources().unwrap();
    service.update_source_selection(selection()).unwrap();
    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    service.start(None, AudioCaptureMode::Manual).unwrap();

    let disabled = service
        .update_preferences(AudioRecordingPreferences {
            enabled: false,
            ..enabled_preferences()
        })
        .unwrap();
    assert!(!disabled.recording_active);
    assert!(!disabled.preferences.enabled);

    service.update_preferences(enabled_preferences()).unwrap();
    service.start(None, AudioCaptureMode::Manual).unwrap();
    let source_disabled = service
        .update_source_selection(AudioSourceSelection {
            enabled: false,
            ..selection()
        })
        .unwrap();
    assert!(!source_disabled.recording_active);
    assert!(!source_disabled.sources[0].enabled);
}

#[test]
fn tombstoned_deletion_is_hidden_and_retried_during_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let media = EncryptedAudioMediaStore::new(
        directory.path(),
        crate::AudioMediaKey::from_test_bytes([12; 32]),
    );
    let service =
        AudioRecordingService::new(store.clone(), media, Some(Arc::new(FakeProvider::new())));
    service.update_preferences(enabled_preferences()).unwrap();
    service.refresh_sources().unwrap();
    service.update_source_selection(selection()).unwrap();
    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    let session_id = service
        .start(None, AudioCaptureMode::Manual)
        .unwrap()
        .active_session_id
        .unwrap();
    service.stop().unwrap();

    store
        .mark_audio_session_tombstoned(&session_id, &timestamp())
        .unwrap();
    assert!(service.status().unwrap().sessions.is_empty());
    assert_eq!(store.list_audio_session_records().unwrap().len(), 1);

    service.recover_interrupted_sessions().unwrap();
    assert!(store.list_audio_session_records().unwrap().is_empty());
}

#[test]
fn provider_batch_cannot_substitute_a_requested_track_source() {
    let provider = FakeProvider::new();
    provider
        .request_permission("synthetic.microphone.primary")
        .unwrap();
    let requests = vec![AudioTrackRequest {
        track_id: "track-1".into(),
        source_id: "synthetic.microphone.primary".into(),
        profile: AudioOpusProfileId::PilotMicrophoneV1,
    }];
    let mut batch = provider
        .start_capture("audio-session-1", &requests)
        .unwrap();
    batch.tracks[0].source_id = "synthetic.microphone.other".into();

    assert_eq!(
        validate_capture_batch("audio-session-1", &requests, &batch).unwrap_err(),
        AudioRecordingError::ProviderFailed
    );
}

#[test]
fn media_cleanup_failure_does_not_block_interrupted_session_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let media_path = directory.path().join("audio-media-v1");
    std::fs::write(&media_path, b"not a directory").unwrap();
    let store = Store::open_in_memory().unwrap();
    let session = AudioSessionRecord {
        id: "audio-session-recovery".into(),
        simulator_session_id: None,
        provider_id: "dev.wyrmgrid.fake-audio".into(),
        capture_mode: "manual".into(),
        started_at: "2026-07-20T00:00:00Z".into(),
        ended_at: None,
        host_start_monotonic_ns: None,
        status: "active".into(),
        media_availability: "available".into(),
        total_media_bytes: 0,
        deletion_requested_at: None,
    };
    let track = AudioTrackRecord {
        id: "audio-track-recovery".into(),
        session_id: session.id.clone(),
        source_id: "synthetic.microphone.primary".into(),
        profile_id: "pilot_microphone_v1".into(),
        source_role: "microphone_input".into(),
        source_truth: "isolated".into(),
        channel_count: 1,
        sample_rate_hz: 48_000,
        provider_start_monotonic_ns: 100,
        packet_count: 0,
        frame_count: 0,
        last_packet_sequence: None,
    };
    store
        .create_audio_session_record(&session, &[track])
        .unwrap();
    let service = AudioRecordingService::new(
        store,
        EncryptedAudioMediaStore::new(media_path, crate::AudioMediaKey::from_test_bytes([13; 32])),
        None,
    );

    let status = service.recover_interrupted_sessions().unwrap();
    assert_eq!(status.sessions[0].status, AudioSessionStatus::Interrupted);
    assert_eq!(
        status.last_code.as_deref(),
        Some("audio.recovery_incomplete")
    );
}

#[test]
fn backup_omission_metadata_does_not_consume_the_local_media_budget() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    for (id, started_at, availability, bytes) in [
        (
            "audio-local",
            "2026-07-20T00:00:00Z",
            "available",
            8 * 1024 * 1024,
        ),
        (
            "audio-backup-only",
            "2026-07-20T00:01:00Z",
            "not_in_backup",
            16 * 1024 * 1024,
        ),
    ] {
        let session = AudioSessionRecord {
            id: id.into(),
            simulator_session_id: None,
            provider_id: "dev.wyrmgrid.fake-audio".into(),
            capture_mode: "manual".into(),
            started_at: started_at.into(),
            ended_at: Some(started_at.into()),
            host_start_monotonic_ns: None,
            status: "completed".into(),
            media_availability: availability.into(),
            total_media_bytes: bytes,
            deletion_requested_at: None,
        };
        store
            .create_audio_session_record(
                &session,
                &[AudioTrackRecord {
                    id: format!("track-{id}"),
                    session_id: id.into(),
                    source_id: "synthetic.microphone.primary".into(),
                    profile_id: "pilot_microphone_v1".into(),
                    source_role: "microphone_input".into(),
                    source_truth: "isolated".into(),
                    channel_count: 1,
                    sample_rate_hz: 48_000,
                    provider_start_monotonic_ns: 100,
                    packet_count: 0,
                    frame_count: 0,
                    last_packet_sequence: None,
                }],
            )
            .unwrap();
    }
    store
        .save_audio_recording_preferences_record(&AudioRecordingPreferencesRecord {
            enabled: false,
            capture_manual: false,
            capture_automatic: false,
            retention_days: 3_650,
            storage_budget_bytes: 16 * 1024 * 1024,
        })
        .unwrap();
    let service = AudioRecordingService::new(
        store,
        EncryptedAudioMediaStore::new(
            directory.path(),
            crate::AudioMediaKey::from_test_bytes([15; 32]),
        ),
        None,
    );

    assert_eq!(service.enforce_retention().unwrap().sessions.len(), 2);
}

#[test]
fn restored_backup_metadata_does_not_retain_an_external_segment() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let session = AudioSessionRecord {
        id: "audio-restored".into(),
        simulator_session_id: None,
        provider_id: "dev.wyrmgrid.fake-audio".into(),
        capture_mode: "manual".into(),
        started_at: "2026-07-20T00:00:00Z".into(),
        ended_at: Some("2026-07-20T00:01:00Z".into()),
        host_start_monotonic_ns: None,
        status: "completed".into(),
        media_availability: "not_in_backup".into(),
        total_media_bytes: 0,
        deletion_requested_at: None,
    };
    let track = AudioTrackRecord {
        id: "track-restored".into(),
        session_id: session.id.clone(),
        source_id: "synthetic.microphone.primary".into(),
        profile_id: "pilot_microphone_v1".into(),
        source_role: "microphone_input".into(),
        source_truth: "isolated".into(),
        channel_count: 1,
        sample_rate_hz: 48_000,
        provider_start_monotonic_ns: 100,
        packet_count: 0,
        frame_count: 0,
        last_packet_sequence: None,
    };
    store
        .create_audio_session_record(&session, std::slice::from_ref(&track))
        .unwrap();
    let storage_key = "aabbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    store
        .complete_audio_segment_record(
            &AudioSegmentRecord {
                track_id: track.id,
                segment_index: 0,
                storage_key: storage_key.into(),
                first_frame: 0,
                frame_count: 960,
                packet_count: 1,
                encrypted_bytes: 96,
                envelope_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .into(),
                envelope_version: 1,
                key_version: 1,
                state: "unavailable".into(),
                created_at: "2026-07-20T00:00:00Z".into(),
                deletion_requested_at: None,
            },
            1,
        )
        .unwrap();
    let segment_directory = directory.path().join("aa");
    std::fs::create_dir_all(&segment_directory).unwrap();
    let segment_path = segment_directory.join(format!("{storage_key}.wga"));
    std::fs::write(&segment_path, b"stale external media").unwrap();
    let service = AudioRecordingService::new(
        store,
        EncryptedAudioMediaStore::new(
            directory.path(),
            crate::AudioMediaKey::from_test_bytes([16; 32]),
        ),
        None,
    );

    let recovered = service.recover_interrupted_sessions().unwrap();
    assert_eq!(recovered.sessions.len(), 1);
    assert!(!segment_path.exists());
}

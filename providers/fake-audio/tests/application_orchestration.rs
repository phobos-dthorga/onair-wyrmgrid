use std::sync::Arc;

use wyrmgrid_application::{
    AudioCaptureMode, AudioCaptureProvider, AudioMediaKey, AudioProviderRegistration,
    AudioRecordingPreferences, AudioRecordingService, AudioSourceSelection,
    EncryptedAudioMediaStore, FakeAudioProviderProcess,
};
use wyrmgrid_domain::AudioOpusProfileId;
use wyrmgrid_storage::{DatabaseKey, Store};

#[test]
fn application_orchestrates_the_development_provider_without_implicit_permission() {
    let executable = env!("CARGO_BIN_EXE_wyrmgrid-fake-audio-provider");
    let registration =
        AudioProviderRegistration::from_manifest_json(include_str!("../provider.json"), executable)
            .unwrap();
    let provider = Arc::new(FakeAudioProviderProcess::new(registration).unwrap());
    let initial_sources = provider.sources().unwrap();
    assert_eq!(initial_sources.len(), 2);
    assert!(!initial_sources[0].is_capture_ready());

    let media_directory = tempfile::tempdir().unwrap();
    let media_key = AudioMediaKey::derive(&DatabaseKey::from_bytes([23; 32])).unwrap();
    let service = AudioRecordingService::new(
        Store::open_in_memory().unwrap(),
        EncryptedAudioMediaStore::new(media_directory.path(), media_key),
        Some(provider),
    );
    service
        .update_preferences(AudioRecordingPreferences {
            enabled: true,
            capture_manual: true,
            capture_automatic: false,
            retention_days: 30,
            storage_budget_bytes: 64 * 1024 * 1024,
        })
        .unwrap();
    service.refresh_sources().unwrap();
    service
        .update_source_selection(AudioSourceSelection {
            provider_id: "dev.wyrmgrid.fake-audio".into(),
            source_id: "synthetic.microphone.primary".into(),
            profile_id: AudioOpusProfileId::PilotMicrophoneV1,
            enabled: true,
            playback_muted: false,
            playback_solo: false,
            playback_volume_percent: 100,
        })
        .unwrap();
    assert!(service.start(None, AudioCaptureMode::Manual).is_err());

    service
        .request_source_permission("synthetic.microphone.primary")
        .unwrap();
    let active = service.start(None, AudioCaptureMode::Manual).unwrap();
    assert!(active.recording_active);
    let session_id = active.active_session_id.unwrap();
    service.stop().unwrap();
    let playback = service.playback(&session_id).unwrap();
    assert!(playback.authenticated);
    assert_eq!(
        playback.tracks[0].packets[0].bytes,
        vec![0xf8, 0xff, 0xfe, 0x00]
    );
}

use std::sync::Arc;

use wyrmgrid_application::{
    AudioCaptureMode, AudioCaptureProvider, AudioCodecError, AudioCodecProvider, AudioMediaKey,
    AudioProviderPcmFrame, AudioProviderRegistration, AudioRecordingPreferences,
    AudioRecordingService, AudioSourceSelection, EncodedAudioPacket, EncryptedAudioMediaStore,
    FakeAudioProviderProcess,
};
use wyrmgrid_audio_codec_protocol::AudioCodecProfile;
use wyrmgrid_domain::AudioProfileId;
use wyrmgrid_storage::{DatabaseKey, Store};

struct TestCodec {
    profiles: Vec<AudioCodecProfile>,
}

impl TestCodec {
    fn new() -> Self {
        Self {
            profiles: vec![AudioCodecProfile {
                id: AudioProfileId::PilotMicrophoneV1,
                codec_id: "test".into(),
                media_type: "application/vnd.wyrmgrid.test-audio".into(),
                channels: 1,
                sample_rate_hz: 48_000,
                target_bitrate_bps: 24_000,
                packet_duration_48khz_frames: 960,
            }],
        }
    }
}

impl AudioCodecProvider for TestCodec {
    fn provider_id(&self) -> &str {
        "dev.wyrmgrid.test-codec"
    }

    fn provider_version(&self) -> &str {
        "0.3.1"
    }

    fn display_name(&self) -> &str {
        "Test codec"
    }

    fn profiles(&self) -> &[AudioCodecProfile] {
        &self.profiles
    }

    fn start_track(
        &self,
        _session_id: &str,
        _track_id: &str,
        _profile: AudioProfileId,
    ) -> Result<(), AudioCodecError> {
        Ok(())
    }

    fn encode_pcm(
        &self,
        frame: &AudioProviderPcmFrame,
    ) -> Result<EncodedAudioPacket, AudioCodecError> {
        Ok(EncodedAudioPacket {
            sequence: frame.frame_sequence,
            provider_monotonic_ns: frame.provider_monotonic_ns,
            duration_48khz_frames: frame.frame_count,
            bytes: vec![0xf8, 0xff, 0xfe, 0x00],
        })
    }

    fn stop_track(&self, _session_id: &str, _track_id: &str) -> Result<(), AudioCodecError> {
        Ok(())
    }
}

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
        vec![Arc::new(TestCodec::new())],
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
            profile_id: AudioProfileId::PilotMicrophoneV1,
            codec_provider_id: "dev.wyrmgrid.test-codec".into(),
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
    service.poll_active_capture().unwrap();
    service.stop().unwrap();
    let playback = service.playback(&session_id).unwrap();
    assert!(playback.authenticated);
    assert_eq!(
        playback.tracks[0].packets[0].bytes,
        vec![0xf8, 0xff, 0xfe, 0x00]
    );
}

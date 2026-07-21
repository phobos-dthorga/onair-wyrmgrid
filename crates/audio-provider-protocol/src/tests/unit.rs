use super::*;
use wyrmgrid_domain::{
    AUDIO_SOURCE_SCHEMA_VERSION, AudioProfileId, AudioSourceAvailability, AudioSourceCapability,
    AudioSourceDirection, AudioSourceOrigin, AudioSourceRole, AudioSourceTruth,
};

fn manifest() -> AudioProviderManifest {
    serde_json::from_str(include_str!(
        "../../../../providers/fake-audio/provider.json"
    ))
    .expect("fake audio provider manifest should deserialize")
}

fn track() -> AudioTrackRequest {
    AudioTrackRequest {
        track_id: "pilot-microphone".into(),
        source_id: "synthetic.microphone.primary".into(),
        profile: AudioProfileId::PilotMicrophoneV1,
    }
}

fn microphone() -> AudioSourceCapability {
    AudioSourceCapability {
        schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
        id: "synthetic.microphone.primary".into(),
        display_name: "Synthetic pilot microphone".into(),
        role: AudioSourceRole::MicrophoneInput,
        direction: AudioSourceDirection::Input,
        truth: AudioSourceTruth::Isolated,
        availability: AudioSourceAvailability::Available,
        permission: wyrmgrid_domain::AudioPermissionState::Granted,
        channels: 1,
        native_sample_rate_hz: 48_000,
        supported_profiles: vec![AudioProfileId::PilotMicrophoneV1],
        supports_hot_plug: true,
        origin: AudioSourceOrigin::OperatingSystem,
    }
}

fn pcm_frame(frame_count: u16) -> AudioProviderMessage {
    AudioProviderMessage::PcmFrame {
        session_id: "session-fixture-1".into(),
        track_id: "pilot-microphone".into(),
        frame_sequence: 1,
        provider_monotonic_ns: 1_020_000,
        channels: 1,
        sample_rate_hz: 48_000,
        frame_count,
        payload_bytes: u32::from(frame_count) * 2,
    }
}

fn gap(affected_frames: Option<u64>) -> AudioProviderMessage {
    AudioProviderMessage::CaptureEvent {
        session_id: "session-fixture-1".into(),
        track_id: Some("pilot-microphone".into()),
        provider_monotonic_ns: 1_040_000,
        event: AudioCaptureEventKind::Gap,
        code: "capture.synthetic_gap".into(),
        affected_frames,
        drift_parts_per_million: None,
    }
}

#[test]
fn validates_the_development_only_provider_manifest() {
    assert_eq!(manifest().validate(), Ok(()));

    let mut unsafe_manifest = manifest();
    unsafe_manifest.entry_point = "../outside.exe".into();
    assert_eq!(
        unsafe_manifest.validate(),
        Err(AudioProviderManifestError::UnsafeEntryPoint)
    );

    let mut current_directory = manifest();
    current_directory.entry_point = ".".into();
    assert_eq!(
        current_directory.validate(),
        Err(AudioProviderManifestError::UnsafeEntryPoint)
    );

    let mut duplicate = manifest();
    duplicate.capabilities.push(duplicate.capabilities[0]);
    assert_eq!(
        duplicate.validate(),
        Err(AudioProviderManifestError::InvalidDeclaration)
    );
}

#[test]
fn validates_host_messages_and_rejects_duplicate_tracks_or_sources() {
    let start = AudioHostMessage::StartCapture {
        session_id: "session-fixture-1".into(),
        tracks: vec![track()],
    };
    assert_eq!(start.validate(), Ok(()));
    assert_eq!(
        AudioHostMessage::DrainCapture {
            session_id: "session-fixture-1".into(),
            maximum_frames: MAX_AUDIO_DRAIN_FRAMES,
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        AudioHostMessage::DrainCapture {
            session_id: "session-fixture-1".into(),
            maximum_frames: 0,
        }
        .validate(),
        Err(AudioMessageError::InvalidHostMessage)
    );
    assert_eq!(
        AudioHostMessage::RequestPermission {
            source_id: "synthetic.microphone.primary".into(),
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        AudioHostMessage::RequestPermission {
            source_id: String::new(),
        }
        .validate(),
        Err(AudioMessageError::InvalidHostMessage)
    );

    let duplicate_track = AudioHostMessage::StartCapture {
        session_id: "session-fixture-1".into(),
        tracks: vec![track(), track()],
    };
    assert_eq!(
        duplicate_track.validate(),
        Err(AudioMessageError::InvalidHostMessage)
    );

    assert_eq!(
        AudioHostMessage::SynchronizeClock {
            request_id: 0,
            host_send_monotonic_ns: 0,
        }
        .validate(),
        Err(AudioMessageError::InvalidHostMessage)
    );
}

#[test]
fn validates_provider_sources_pcm_frames_levels_and_events() {
    let sources = AudioProviderMessage::Sources {
        revision: 1,
        sources: vec![microphone()],
    };
    assert_eq!(sources.validate(), Ok(()));
    assert_eq!(
        AudioProviderMessage::Sources {
            revision: 1,
            sources: vec![],
        }
        .validate(),
        Ok(())
    );

    let mut duplicate_sources = vec![microphone(), microphone()];
    duplicate_sources[1].display_name = "Another synthetic label".into();
    assert_eq!(
        AudioProviderMessage::Sources {
            revision: 1,
            sources: duplicate_sources,
        }
        .validate(),
        Err(AudioMessageError::InvalidSources)
    );

    assert_eq!(pcm_frame(960).validate(), Ok(()));
    let invalid_duration = AudioProviderMessage::PcmFrame {
        session_id: "session-fixture-1".into(),
        track_id: "pilot-microphone".into(),
        frame_sequence: 1,
        provider_monotonic_ns: 1_020_000,
        channels: 1,
        sample_rate_hz: 48_000,
        frame_count: 720,
        payload_bytes: 1_440,
    };
    assert_eq!(
        invalid_duration.validate(),
        Err(AudioMessageError::InvalidPacket)
    );

    let invalid_level = AudioProviderMessage::Level {
        session_id: "session-fixture-1".into(),
        track_id: "pilot-microphone".into(),
        provider_monotonic_ns: 1_020_000,
        peak_millidbfs: 1,
        clipped: true,
    };
    assert_eq!(
        invalid_level.validate(),
        Err(AudioMessageError::InvalidLevel)
    );

    assert_eq!(gap(Some(960)).validate(), Ok(()));
    assert_eq!(
        AudioProviderMessage::DrainComplete {
            session_id: "session-fixture-1".into(),
            frame_count: 1,
        }
        .validate(),
        Ok(())
    );

    let malformed_gap = gap(None);
    assert_eq!(
        malformed_gap.validate(),
        Err(AudioMessageError::InvalidEvent)
    );
}

#[test]
fn round_trips_bounded_host_and_provider_frames() {
    let host = AudioEnvelope::new(
        1,
        AudioHostMessage::Hello {
            host_version: "0.2.0".into(),
            provider_id: manifest().id,
        },
    );
    let mut bytes = Vec::new();
    write_host_frame(&mut bytes, &host).expect("host frame should encode");
    assert_eq!(
        read_host_frame(&mut bytes.as_slice()).expect("host frame should decode"),
        host
    );

    let body = vec![0_u8; 1_920];
    let provider = AudioEnvelope::new(2, pcm_frame(960));
    bytes.clear();
    write_provider_frame(&mut bytes, &provider, &body).expect("provider frame should encode");
    assert_eq!(
        read_provider_frame(&mut bytes.as_slice()).expect("provider frame should decode"),
        (provider, body)
    );
}

#[test]
fn rejects_oversized_or_inconsistent_bodies_before_payload_allocation() {
    let oversized = [
        1_u32.to_be_bytes(),
        ((MAX_PCM_AUDIO_FRAME_BYTES as u32) + 1).to_be_bytes(),
    ]
    .concat();
    assert!(matches!(
        read_provider_frame(&mut oversized.as_slice()),
        Err(AudioFrameError::BodyTooLarge { .. })
    ));

    let control = AudioEnvelope::new(
        1,
        AudioProviderMessage::State {
            state: AudioProviderState::Ready,
            code: "provider.ready".into(),
        },
    );
    assert!(matches!(
        write_provider_frame(&mut Vec::new(), &control, &[1]),
        Err(AudioFrameError::UnexpectedBody)
    ));

    let declared = AudioEnvelope::new(1, pcm_frame(120));
    assert!(matches!(
        write_provider_frame(&mut Vec::new(), &declared, &[]),
        Err(AudioFrameError::MissingBody)
    ));
    assert!(matches!(
        write_provider_frame(&mut Vec::new(), &declared, &[1, 2]),
        Err(AudioFrameError::BodyLengthMismatch)
    ));
}

#[test]
fn rejects_truncated_and_malformed_host_frames() {
    assert!(matches!(
        read_host_frame(&mut [0_u8, 0].as_slice()),
        Err(AudioFrameError::TruncatedHeader)
    ));

    let malformed_utf8 = [4_u32.to_be_bytes().as_slice(), &[0xff, 0xff, 0xff, 0xff]].concat();
    assert!(matches!(
        read_host_frame(&mut malformed_utf8.as_slice()),
        Err(AudioFrameError::Decode(_))
    ));
}

#[test]
fn validates_sanitized_version_two_fixtures() {
    let host: AudioEnvelope<AudioHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-host-hello-v2.json"
    ))
    .expect("host fixture should deserialize");
    host.validate_header().expect("header should validate");
    host.payload.validate().expect("message should validate");

    let permission: AudioEnvelope<AudioHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-request-permission-v2.json"
    ))
    .expect("permission fixture should deserialize");
    permission
        .validate_header()
        .expect("permission header should validate");
    permission
        .payload
        .validate()
        .expect("permission message should validate");

    let provider: AudioEnvelope<AudioProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-hello-v2.json"
    ))
    .expect("provider fixture should deserialize");
    provider.validate_header().expect("header should validate");
    provider
        .payload
        .validate()
        .expect("message should validate");

    let sources: AudioEnvelope<AudioProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-sources-v2.json"
    ))
    .expect("sources fixture should deserialize");
    sources.payload.validate().expect("sources should validate");

    let clock: AudioEnvelope<AudioProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-clock-v2.json"
    ))
    .expect("clock fixture should deserialize");
    clock.payload.validate().expect("clock should validate");

    let frame_header: AudioEnvelope<AudioProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-pcm-frame-header-v2.json"
    ))
    .expect("PCM frame fixture should deserialize");
    frame_header
        .payload
        .validate()
        .expect("PCM frame metadata should validate");
    let frame_body =
        include_str!("../../../../schemas/fixtures/audio-provider-pcm-frame-body-v2.hex").trim();
    assert_eq!(frame_body.len(), 480);
    assert!(frame_body.chars().all(|character| character == '0'));

    let event: AudioEnvelope<AudioProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-capture-event-v2.json"
    ))
    .expect("event fixture should deserialize");
    event.payload.validate().expect("event should validate");
}

#[test]
fn rejects_unknown_fields_and_non_increasing_sequences() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-provider-host-hello-v2.json"
    ))
    .expect("fixture should parse");
    value["payload"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<AudioEnvelope<AudioHostMessage>>(value).is_err());

    assert!(validate_next_sequence(1, 2));
    assert!(!validate_next_sequence(1, 1));
    assert!(!validate_next_sequence(2, 1));
}

#[test]
fn keeps_schema_limits_aligned_with_the_rust_contract() {
    let envelope_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/audio-provider-envelope-v2.schema.json"
    ))
    .expect("envelope schema should be valid JSON");
    assert_eq!(
        envelope_schema["properties"]["protocol_version"]["const"],
        AUDIO_PROVIDER_PROTOCOL_VERSION
    );
    assert_eq!(
        envelope_schema["$defs"]["sources"]["properties"]["sources"]["maxItems"],
        MAX_AUDIO_SOURCES
    );
    assert_eq!(
        envelope_schema["$defs"]["startCapture"]["properties"]["tracks"]["maxItems"],
        MAX_AUDIO_TRACKS
    );
    assert_eq!(
        envelope_schema["$defs"]["pcmFrame"]["properties"]["payload_bytes"]["maximum"],
        MAX_PCM_AUDIO_FRAME_BYTES
    );

    let source_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/audio-source-capability-v1.schema.json"
    ))
    .expect("source schema should be valid JSON");
    assert_eq!(
        source_schema["properties"]["schema_version"]["const"],
        wyrmgrid_domain::AUDIO_SOURCE_SCHEMA_VERSION
    );
    assert_eq!(
        source_schema["properties"]["channels"]["maximum"],
        wyrmgrid_domain::MAX_AUDIO_SOURCE_CHANNELS
    );

    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/audio-provider-manifest-v2.schema.json"
    ))
    .expect("manifest schema should be valid JSON");
    assert_eq!(
        manifest_schema["properties"]["schema_version"]["const"],
        AUDIO_PROVIDER_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        manifest_schema["properties"]["audio_protocol_version"]["const"],
        AUDIO_PROVIDER_PROTOCOL_VERSION
    );
    assert_eq!(
        manifest_schema["properties"]["capabilities"]["maxItems"],
        MAX_AUDIO_PROVIDER_CAPABILITIES
    );
    assert_eq!(
        envelope_schema["$defs"]["providerDescriptor"]["properties"]["capabilities"]["maxItems"],
        MAX_AUDIO_PROVIDER_CAPABILITIES
    );
}

use std::io::Cursor;

use crate::*;

#[test]
fn validates_codec_manifests_and_profile_estimates() {
    let manifest: AudioCodecManifest =
        serde_json::from_str(include_str!("../../../../codecs/opus/codec.json"))
            .expect("bundled Opus manifest should deserialize");
    assert_eq!(manifest.validate(), Ok(()));
    assert_eq!(
        manifest.profiles[0].estimated_encoded_bytes(3_600),
        Some(21_600_000)
    );
}

#[test]
fn validates_sanitized_codec_fixtures_and_schema_limits() {
    let hello: CodecEnvelope<AudioCodecHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-codec-host-hello-v1.json"
    ))
    .unwrap();
    hello.validate_header().unwrap();
    hello.payload.validate().unwrap();

    let start: CodecEnvelope<AudioCodecHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-codec-start-track-v1.json"
    ))
    .unwrap();
    start.payload.validate().unwrap();

    let pcm: CodecEnvelope<AudioCodecHostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-codec-pcm-header-v1.json"
    ))
    .unwrap();
    pcm.payload.validate().unwrap();
    assert_eq!(
        include_str!("../../../../schemas/fixtures/audio-codec-pcm-body-v1.hex")
            .trim()
            .len(),
        480
    );

    let encoded: CodecEnvelope<AudioCodecProviderMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/audio-codec-encoded-header-v1.json"
    ))
    .unwrap();
    encoded.payload.validate().unwrap();
    assert_eq!(
        include_str!("../../../../schemas/fixtures/audio-codec-encoded-body-v1.hex").trim(),
        "0102030405060708"
    );

    let envelope_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/audio-codec-envelope-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        envelope_schema["properties"]["protocol_version"]["const"],
        AUDIO_CODEC_PROTOCOL_VERSION
    );
    assert_eq!(
        envelope_schema["$defs"]["encodePcm"]["properties"]["payload_bytes"]["maximum"],
        MAX_CODEC_PCM_FRAME_BYTES
    );
    assert_eq!(
        envelope_schema["$defs"]["encodedPacket"]["properties"]["payload_bytes"]["maximum"],
        MAX_CODEC_PACKET_BYTES
    );

    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/audio-codec-manifest-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        manifest_schema["properties"]["schema_version"]["const"],
        AUDIO_CODEC_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(
        manifest_schema["properties"]["profiles"]["maxItems"],
        MAX_CODEC_PROFILES
    );
}

#[test]
fn round_trips_bounded_pcm_and_encoded_bodies() {
    let pcm = vec![0_u8; 960 * 2];
    let host = CodecEnvelope::new(
        1,
        AudioCodecHostMessage::EncodePcm {
            session_id: "audio-session".into(),
            track_id: "track-1".into(),
            frame_sequence: 1,
            provider_monotonic_ns: 42,
            sample_format: PcmSampleFormat::S16le,
            channels: 1,
            sample_rate_hz: 48_000,
            frame_count: 960,
            payload_bytes: pcm.len() as u32,
        },
    );
    let mut bytes = Vec::new();
    write_codec_host_frame(&mut bytes, &host, &pcm).unwrap();
    let (decoded, body) = read_codec_host_frame(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(decoded, host);
    assert_eq!(body, pcm);

    let packet = vec![1_u8; 80];
    let provider = CodecEnvelope::new(
        1,
        AudioCodecProviderMessage::EncodedPacket {
            session_id: "audio-session".into(),
            track_id: "track-1".into(),
            packet_sequence: 1,
            provider_monotonic_ns: 42,
            duration_48khz_frames: 960,
            payload_bytes: packet.len() as u32,
        },
    );
    let mut bytes = Vec::new();
    write_codec_provider_frame(&mut bytes, &provider, &packet).unwrap();
    let (decoded, body) = read_codec_provider_frame(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(decoded, provider);
    assert_eq!(body, packet);
}

#[test]
fn rejects_pcm_body_mismatches_before_io() {
    let message = CodecEnvelope::new(
        1,
        AudioCodecHostMessage::EncodePcm {
            session_id: "audio-session".into(),
            track_id: "track-1".into(),
            frame_sequence: 1,
            provider_monotonic_ns: 42,
            sample_format: PcmSampleFormat::S16le,
            channels: 1,
            sample_rate_hz: 48_000,
            frame_count: 960,
            payload_bytes: 1_920,
        },
    );
    assert!(write_codec_host_frame(&mut Vec::new(), &message, &[0; 8]).is_err());
}

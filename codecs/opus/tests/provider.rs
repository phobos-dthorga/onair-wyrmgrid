use opus2::{Channels, Decoder};
use std::io::{BufReader, Write};
use std::process::{Command, Stdio};
use wyrmgrid_audio_codec_protocol::{
    AudioCodecHostMessage, AudioCodecProviderMessage, CodecEnvelope, PcmSampleFormat,
    read_codec_provider_frame, write_codec_host_frame,
};
use wyrmgrid_domain::AudioProfileId;

#[test]
fn encodes_a_synthetic_pcm_frame_through_the_plugin_protocol() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wyrmgrid-opus-codec"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Opus codec should start");
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut input,
        1,
        AudioCodecHostMessage::Hello {
            host_version: "0.3.1".into(),
            codec_provider_id: "dev.wyrmgrid.opus".into(),
        },
        &[],
    );
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::State { .. }
    ));
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::Hello { .. }
    ));
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::State { .. }
    ));

    send(
        &mut input,
        2,
        AudioCodecHostMessage::StartTrack {
            session_id: "audio-synthetic".into(),
            track_id: "track-1".into(),
            profile: AudioProfileId::PilotMicrophoneV1,
        },
        &[],
    );
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::TrackStarted { .. }
    ));

    let pcm = vec![0_u8; 1_920];
    send(
        &mut input,
        3,
        AudioCodecHostMessage::EncodePcm {
            session_id: "audio-synthetic".into(),
            track_id: "track-1".into(),
            frame_sequence: 1,
            provider_monotonic_ns: 42,
            sample_format: PcmSampleFormat::S16le,
            channels: 1,
            sample_rate_hz: 48_000,
            frame_count: 960,
            payload_bytes: pcm.len() as u32,
        },
        &pcm,
    );
    let (message, packet) = receive(&mut output);
    assert!(matches!(
        message,
        AudioCodecProviderMessage::EncodedPacket {
            packet_sequence: 1,
            duration_48khz_frames: 960,
            ..
        }
    ));
    assert!(!packet.is_empty());

    let mut decoder = Decoder::new(48_000, Channels::Mono).unwrap();
    assert_eq!(decoder.get_nb_samples(&packet).unwrap(), 960);
    let mut decoded = vec![0_i16; 960];
    assert_eq!(decoder.decode(&packet, &mut decoded, false).unwrap(), 960);

    send(
        &mut input,
        4,
        AudioCodecHostMessage::StopTrack {
            session_id: "audio-synthetic".into(),
            track_id: "track-1".into(),
        },
        &[],
    );
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::TrackStopped { .. }
    ));
    send(&mut input, 5, AudioCodecHostMessage::Shutdown, &[]);
    assert!(matches!(
        receive(&mut output).0,
        AudioCodecProviderMessage::State { .. }
    ));
    drop(input);
    assert!(child.wait().unwrap().success());
}

fn send(input: &mut impl Write, sequence: u64, message: AudioCodecHostMessage, body: &[u8]) {
    write_codec_host_frame(input, &CodecEnvelope::new(sequence, message), body).unwrap();
}

fn receive(output: &mut impl std::io::Read) -> (AudioCodecProviderMessage, Vec<u8>) {
    let (envelope, body) = read_codec_provider_frame(output).unwrap();
    (envelope.payload, body)
}

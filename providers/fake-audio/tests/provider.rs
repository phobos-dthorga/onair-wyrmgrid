use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use wyrmgrid_audio_provider_protocol::{
    AudioCaptureEventKind, AudioEnvelope, AudioHostMessage, AudioProviderMessage,
    AudioProviderState, AudioStopReason, AudioTrackRequest, read_provider_frame, write_host_frame,
};
use wyrmgrid_domain::{AudioOpusProfileId, AudioPermissionState, AudioSourceAvailability};

fn spawn_provider() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wyrmgrid-fake-audio-provider"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("fake provider should start");
    let stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = BufReader::new(child.stdout.take().expect("stdout should be piped"));
    (child, stdin, stdout)
}

fn send(stdin: &mut ChildStdin, sequence: u64, message: AudioHostMessage) {
    write_host_frame(stdin, &AudioEnvelope::new(sequence, message))
        .expect("host command should write");
    stdin.flush().expect("host command should flush");
}

fn receive(stdout: &mut BufReader<ChildStdout>) -> (AudioProviderMessage, Vec<u8>) {
    let (envelope, body) = read_provider_frame(stdout).expect("provider frame should read");
    (envelope.payload, body)
}

fn complete_handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    send(
        stdin,
        1,
        AudioHostMessage::Hello {
            host_version: "0.2.0".into(),
            provider_id: "dev.wyrmgrid.fake-audio".into(),
        },
    );
    assert!(matches!(
        receive(stdout).0,
        AudioProviderMessage::Hello { .. }
    ));
    assert!(matches!(
        receive(stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Starting,
            ..
        }
    ));
    assert!(matches!(
        receive(stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Ready,
            ..
        }
    ));
}

#[test]
fn follows_the_deterministic_capture_lifecycle_without_hardware() {
    let (mut child, mut stdin, mut stdout) = spawn_provider();
    complete_handshake(&mut stdin, &mut stdout);

    send(&mut stdin, 2, AudioHostMessage::EnumerateSources);
    let sources = match receive(&mut stdout).0 {
        AudioProviderMessage::Sources { sources, .. } => sources,
        message => panic!("expected sources, received {message:?}"),
    };
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].id, "synthetic.microphone.primary");
    assert_eq!(sources[0].permission, AudioPermissionState::PromptRequired);
    assert_eq!(
        sources[1].availability,
        AudioSourceAvailability::Unavailable
    );

    send(
        &mut stdin,
        3,
        AudioHostMessage::RequestPermission {
            source_id: "synthetic.microphone.primary".into(),
        },
    );
    let sources = match receive(&mut stdout).0 {
        AudioProviderMessage::Sources {
            revision: 2,
            sources,
        } => sources,
        message => panic!("expected updated sources, received {message:?}"),
    };
    assert_eq!(sources[0].permission, AudioPermissionState::Granted);

    send(
        &mut stdin,
        4,
        AudioHostMessage::SynchronizeClock {
            request_id: 7,
            host_send_monotonic_ns: 500_000,
        },
    );
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::ClockSynchronized {
            request_id: 7,
            host_send_monotonic_ns: 500_000,
            provider_receive_monotonic_ns: 1_000_000,
            provider_send_monotonic_ns: 1_000_100,
        }
    ));

    send(
        &mut stdin,
        5,
        AudioHostMessage::StartCapture {
            session_id: "session-black-box-1".into(),
            tracks: vec![AudioTrackRequest {
                track_id: "pilot-microphone".into(),
                source_id: "synthetic.microphone.primary".into(),
                profile: AudioOpusProfileId::PilotMicrophoneV1,
            }],
        },
    );
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Capturing,
            ..
        }
    ));
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::CaptureStarted { .. }
    ));
    let (packet, body) = receive(&mut stdout);
    assert!(matches!(
        packet,
        AudioProviderMessage::AudioPacket {
            packet_sequence: 1,
            duration_48khz_frames: 960,
            payload_bytes: 4,
            ..
        }
    ));
    assert_eq!(body, [0xf8, 0xff, 0xfe, 0x00]);
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::Level {
            peak_millidbfs: -12_000,
            clipped: false,
            ..
        }
    ));
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::CaptureEvent {
            event: AudioCaptureEventKind::Gap,
            affected_frames: Some(960),
            ..
        }
    ));

    send(
        &mut stdin,
        6,
        AudioHostMessage::StopCapture {
            session_id: "session-black-box-1".into(),
        },
    );
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::CaptureStopped {
            reason: AudioStopReason::UserRequested,
            ..
        }
    ));
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Ready,
            ..
        }
    ));

    send(&mut stdin, 7, AudioHostMessage::Shutdown);
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Stopped,
            ..
        }
    ));
    drop(stdin);
    assert!(child.wait().expect("provider should exit").success());
}

#[test]
fn fails_closed_for_an_unavailable_source_without_fallback() {
    let (mut child, mut stdin, mut stdout) = spawn_provider();
    complete_handshake(&mut stdin, &mut stdout);
    send(
        &mut stdin,
        2,
        AudioHostMessage::StartCapture {
            session_id: "session-unavailable-1".into(),
            tracks: vec![AudioTrackRequest {
                track_id: "simulator-mix".into(),
                source_id: "synthetic.simulator.mix".into(),
                profile: AudioOpusProfileId::MixedStereoV1,
            }],
        },
    );
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Failed,
            ref code,
        } if code == "capture.source_unavailable"
    ));
    drop(stdin);
    assert!(!child.wait().expect("provider should exit").success());
}

#[test]
fn rejects_a_non_increasing_host_sequence() {
    let (mut child, mut stdin, mut stdout) = spawn_provider();
    complete_handshake(&mut stdin, &mut stdout);
    send(&mut stdin, 1, AudioHostMessage::EnumerateSources);
    drop(stdin);
    assert!(!child.wait().expect("provider should exit").success());
}

#[test]
fn rejects_a_stop_for_a_session_that_is_not_active() {
    let (mut child, mut stdin, mut stdout) = spawn_provider();
    complete_handshake(&mut stdin, &mut stdout);
    send(
        &mut stdin,
        2,
        AudioHostMessage::StopCapture {
            session_id: "session-not-active".into(),
        },
    );
    assert!(matches!(
        receive(&mut stdout).0,
        AudioProviderMessage::State {
            state: AudioProviderState::Failed,
            ref code,
        } if code == "capture.session_mismatch"
    ));
    drop(stdin);
    assert!(!child.wait().expect("provider should exit").success());
}

use std::io::{BufWriter, Stdin, Stdout};
use wyrmgrid_audio_provider_protocol::{
    AudioCaptureEventKind, AudioEnvelope, AudioHostMessage, AudioProviderCapability,
    AudioProviderDescriptor, AudioProviderMessage, AudioProviderPlatform, AudioProviderState,
    AudioStartedTrack, AudioStopReason, read_host_frame, validate_next_sequence,
    write_provider_frame,
};
use wyrmgrid_domain::{
    AUDIO_SOURCE_SCHEMA_VERSION, AudioPermissionState, AudioProfileId, AudioSourceAvailability,
    AudioSourceCapability, AudioSourceDirection, AudioSourceOrigin, AudioSourceRole,
    AudioSourceTruth,
};

const PROVIDER_ID: &str = "dev.wyrmgrid.fake-audio";
const MICROPHONE_SOURCE_ID: &str = "synthetic.microphone.primary";
const MIX_SOURCE_ID: &str = "synthetic.simulator.mix";
const SYNTHETIC_PCM_FRAME: [u8; 1_920] = [0; 1_920];

struct ActiveCapture {
    session_id: String,
    tracks: Vec<AudioStartedTrack>,
    synthetic_frame_sent: bool,
}

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let mut stdin = std::io::stdin();
    let first = read_next(&mut stdin, 0)?;
    match first.payload {
        AudioHostMessage::Hello { provider_id, .. } if provider_id == PROVIDER_ID => {}
        _ => return Err(()),
    }

    let mut writer = ProviderWriter::new(std::io::stdout());
    writer.send(
        AudioProviderMessage::Hello {
            provider: descriptor(),
        },
        &[],
    )?;
    writer.send(
        AudioProviderMessage::State {
            state: AudioProviderState::Starting,
            code: "provider.starting".into(),
        },
        &[],
    )?;
    writer.send(
        AudioProviderMessage::State {
            state: AudioProviderState::Ready,
            code: "provider.ready".into(),
        },
        &[],
    )?;

    let mut last_sequence = first.sequence;
    let mut active_capture: Option<ActiveCapture> = None;
    let mut microphone_permission_granted = false;
    loop {
        let envelope = read_next(&mut stdin, last_sequence)?;
        last_sequence = envelope.sequence;
        match envelope.payload {
            AudioHostMessage::EnumerateSources => writer.send(
                AudioProviderMessage::Sources {
                    revision: 1,
                    sources: sources(microphone_permission_granted),
                },
                &[],
            )?,
            AudioHostMessage::RequestPermission { source_id }
                if source_id == MICROPHONE_SOURCE_ID =>
            {
                microphone_permission_granted = true;
                writer.send(
                    AudioProviderMessage::Sources {
                        revision: 2,
                        sources: sources(microphone_permission_granted),
                    },
                    &[],
                )?;
            }
            AudioHostMessage::RequestPermission { .. } => {
                writer.send(
                    AudioProviderMessage::State {
                        state: AudioProviderState::Failed,
                        code: "permission.source_unavailable".into(),
                    },
                    &[],
                )?;
                return Err(());
            }
            AudioHostMessage::SynchronizeClock {
                request_id,
                host_send_monotonic_ns,
            } => writer.send(
                AudioProviderMessage::ClockSynchronized {
                    request_id,
                    host_send_monotonic_ns,
                    provider_receive_monotonic_ns: 1_000_000,
                    provider_send_monotonic_ns: 1_000_100,
                },
                &[],
            )?,
            AudioHostMessage::StartCapture { session_id, tracks } => {
                if active_capture.is_some()
                    || !tracks.iter().all(|track| {
                        source_for(&track.source_id, microphone_permission_granted).is_some_and(
                            |source| {
                                source.is_capture_ready()
                                    && source.supported_profiles.contains(&track.profile)
                            },
                        )
                    })
                {
                    writer.send(
                        AudioProviderMessage::State {
                            state: AudioProviderState::Failed,
                            code: "capture.source_unavailable".into(),
                        },
                        &[],
                    )?;
                    return Err(());
                }
                let started_tracks = tracks
                    .iter()
                    .map(|track| AudioStartedTrack {
                        track_id: track.track_id.clone(),
                        source_id: track.source_id.clone(),
                        profile: track.profile,
                        provider_start_monotonic_ns: 1_010_000,
                    })
                    .collect::<Vec<_>>();
                active_capture = Some(ActiveCapture {
                    session_id: session_id.clone(),
                    tracks: started_tracks.clone(),
                    synthetic_frame_sent: false,
                });
                writer.send(
                    AudioProviderMessage::State {
                        state: AudioProviderState::Capturing,
                        code: "capture.active".into(),
                    },
                    &[],
                )?;
                writer.send(
                    AudioProviderMessage::CaptureStarted {
                        session_id: session_id.clone(),
                        tracks: started_tracks,
                        provider_monotonic_ns: 1_010_000,
                    },
                    &[],
                )?;
            }
            AudioHostMessage::DrainCapture {
                session_id,
                maximum_frames,
            } => {
                let Some(capture) = active_capture
                    .as_mut()
                    .filter(|capture| capture.session_id == session_id)
                else {
                    return Err(());
                };
                let should_send = !capture.synthetic_frame_sent && maximum_frames > 0;
                if should_send {
                    let track_id = capture.tracks[0].track_id.clone();
                    writer.send(
                        AudioProviderMessage::PcmFrame {
                            session_id: session_id.clone(),
                            track_id: track_id.clone(),
                            frame_sequence: 1,
                            provider_monotonic_ns: 1_020_000,
                            channels: 1,
                            sample_rate_hz: 48_000,
                            frame_count: 960,
                            payload_bytes: SYNTHETIC_PCM_FRAME.len() as u32,
                        },
                        &SYNTHETIC_PCM_FRAME,
                    )?;
                    writer.send(
                        AudioProviderMessage::Level {
                            session_id: session_id.clone(),
                            track_id: track_id.clone(),
                            provider_monotonic_ns: 1_020_000,
                            peak_millidbfs: -12_000,
                            clipped: false,
                        },
                        &[],
                    )?;
                    writer.send(
                        AudioProviderMessage::CaptureEvent {
                            session_id: session_id.clone(),
                            track_id: Some(track_id),
                            provider_monotonic_ns: 1_040_000,
                            event: AudioCaptureEventKind::Gap,
                            code: "capture.synthetic_gap".into(),
                            affected_frames: Some(960),
                            drift_parts_per_million: None,
                        },
                        &[],
                    )?;
                    capture.synthetic_frame_sent = true;
                }
                writer.send(
                    AudioProviderMessage::DrainComplete {
                        session_id,
                        frame_count: u16::from(should_send),
                    },
                    &[],
                )?;
            }
            AudioHostMessage::StopCapture { session_id } => {
                if active_capture
                    .as_ref()
                    .map(|capture| capture.session_id.as_str())
                    != Some(session_id.as_str())
                {
                    writer.send(
                        AudioProviderMessage::State {
                            state: AudioProviderState::Failed,
                            code: "capture.session_mismatch".into(),
                        },
                        &[],
                    )?;
                    return Err(());
                }
                active_capture = None;
                writer.send(
                    AudioProviderMessage::CaptureStopped {
                        session_id,
                        provider_monotonic_ns: 1_060_000,
                        reason: AudioStopReason::UserRequested,
                    },
                    &[],
                )?;
                writer.send(
                    AudioProviderMessage::State {
                        state: AudioProviderState::Ready,
                        code: "provider.ready".into(),
                    },
                    &[],
                )?;
            }
            AudioHostMessage::Shutdown => {
                writer.send(
                    AudioProviderMessage::State {
                        state: AudioProviderState::Stopped,
                        code: "provider.stopped".into(),
                    },
                    &[],
                )?;
                return Ok(());
            }
            AudioHostMessage::Hello { .. } => return Err(()),
        }
    }
}

fn read_next(
    stdin: &mut Stdin,
    previous_sequence: u64,
) -> Result<AudioEnvelope<AudioHostMessage>, ()> {
    let envelope = read_host_frame(stdin).map_err(|_| ())?;
    if !validate_next_sequence(previous_sequence, envelope.sequence) {
        return Err(());
    }
    Ok(envelope)
}

fn descriptor() -> AudioProviderDescriptor {
    AudioProviderDescriptor {
        id: PROVIDER_ID.into(),
        name: "WyrmGrid deterministic fake audio provider".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: current_platform(),
        capabilities: vec![
            AudioProviderCapability::SourceEnumeration,
            AudioProviderCapability::PermissionRequests,
            AudioProviderCapability::PcmS16leCapture,
            AudioProviderCapability::LevelMetering,
            AudioProviderCapability::HotPlugNotifications,
            AudioProviderCapability::ClockSynchronization,
        ],
    }
}

fn sources(microphone_permission_granted: bool) -> Vec<AudioSourceCapability> {
    vec![
        AudioSourceCapability {
            schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
            id: MICROPHONE_SOURCE_ID.into(),
            display_name: "Synthetic pilot microphone".into(),
            role: AudioSourceRole::MicrophoneInput,
            direction: AudioSourceDirection::Input,
            truth: AudioSourceTruth::Isolated,
            availability: AudioSourceAvailability::Available,
            permission: if microphone_permission_granted {
                AudioPermissionState::Granted
            } else {
                AudioPermissionState::PromptRequired
            },
            channels: 1,
            native_sample_rate_hz: 48_000,
            supported_profiles: vec![AudioProfileId::PilotMicrophoneV1],
            supports_hot_plug: true,
            origin: AudioSourceOrigin::OperatingSystem,
        },
        AudioSourceCapability {
            schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
            id: MIX_SOURCE_ID.into(),
            display_name: "Synthetic simulator mix".into(),
            role: AudioSourceRole::SimulatorMasterMix,
            direction: AudioSourceDirection::Output,
            truth: AudioSourceTruth::MixedOutput,
            availability: AudioSourceAvailability::Unavailable,
            permission: AudioPermissionState::NotRequired,
            channels: 2,
            native_sample_rate_hz: 48_000,
            supported_profiles: vec![AudioProfileId::MixedStereoV1],
            supports_hot_plug: true,
            origin: AudioSourceOrigin::Simulator {
                identifier: "synthetic_simulator".into(),
            },
        },
    ]
}

fn source_for(
    source_id: &str,
    microphone_permission_granted: bool,
) -> Option<AudioSourceCapability> {
    sources(microphone_permission_granted)
        .into_iter()
        .find(|source| source.id == source_id)
}

fn current_platform() -> AudioProviderPlatform {
    #[cfg(target_os = "windows")]
    return AudioProviderPlatform::WindowsX86_64;
    #[cfg(target_os = "linux")]
    return AudioProviderPlatform::LinuxX86_64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return AudioProviderPlatform::MacosAarch64;
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    return AudioProviderPlatform::MacosX86_64;
}

struct ProviderWriter {
    stdout: BufWriter<Stdout>,
    sequence: u64,
}

impl ProviderWriter {
    fn new(stdout: Stdout) -> Self {
        Self {
            stdout: BufWriter::new(stdout),
            sequence: 1,
        }
    }

    fn send(&mut self, message: AudioProviderMessage, body: &[u8]) -> Result<(), ()> {
        let envelope = AudioEnvelope::new(self.sequence, message);
        write_provider_frame(&mut self.stdout, &envelope, body).map_err(|_| ())?;
        self.sequence += 1;
        Ok(())
    }
}

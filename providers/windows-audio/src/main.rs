#[cfg(target_os = "windows")]
fn main() {
    if windows::run().is_err() {
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("The WyrmGrid Windows audio provider is available only on Windows.");
    std::process::exit(2);
}

#[cfg(target_os = "windows")]
mod windows {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{BufferSize, Device, SampleFormat, Stream, StreamConfig};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    use std::io::{BufWriter, Stdin, Stdout};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, mpsc};
    use std::time::{Duration, Instant};
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

    const PROVIDER_ID: &str = "dev.wyrmgrid.windows-audio";
    const FRAME_COUNT: usize = 960;
    const FRAME_BYTES: usize = FRAME_COUNT * 2;
    const CAPTURE_QUEUE_FRAMES: usize = 64;
    const STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(2);

    pub fn run() -> Result<(), ()> {
        let started = Instant::now();
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
        writer.state(AudioProviderState::Starting, "provider.starting")?;
        writer.state(AudioProviderState::Ready, "provider.ready")?;

        let mut last_sequence = first.sequence;
        let mut permissions = BTreeMap::<String, AudioPermissionState>::new();
        let mut active: Option<ActiveCapture> = None;
        loop {
            let envelope = read_next(&mut stdin, last_sequence)?;
            last_sequence = envelope.sequence;
            match envelope.payload {
                AudioHostMessage::EnumerateSources => {
                    writer.send(
                        AudioProviderMessage::Sources {
                            revision: 1,
                            sources: enumerate_sources(&permissions),
                        },
                        &[],
                    )?;
                }
                AudioHostMessage::RequestPermission { source_id } => {
                    let permission = if find_device(&source_id)
                        .is_some_and(|choice| probe_permission(&choice).is_ok())
                    {
                        AudioPermissionState::Granted
                    } else {
                        AudioPermissionState::Denied
                    };
                    permissions.insert(source_id.clone(), permission);
                    writer.send(
                        AudioProviderMessage::Sources {
                            revision: 2,
                            sources: enumerate_sources(&permissions),
                        },
                        &[],
                    )?;
                }
                AudioHostMessage::SynchronizeClock {
                    request_id,
                    host_send_monotonic_ns,
                } => {
                    let received = monotonic_ns(started);
                    writer.send(
                        AudioProviderMessage::ClockSynchronized {
                            request_id,
                            host_send_monotonic_ns,
                            provider_receive_monotonic_ns: received,
                            provider_send_monotonic_ns: monotonic_ns(started),
                        },
                        &[],
                    )?;
                }
                AudioHostMessage::StartCapture { session_id, tracks } => {
                    if active.is_some() || tracks.len() != 1 {
                        writer.state(AudioProviderState::Failed, "capture.invalid_tracks")?;
                        return Err(());
                    }
                    let track = &tracks[0];
                    if track.profile != AudioProfileId::PilotMicrophoneV1
                        || permissions.get(&track.source_id) != Some(&AudioPermissionState::Granted)
                    {
                        writer.state(AudioProviderState::Failed, "capture.permission_required")?;
                        return Err(());
                    }
                    let choice = find_device(&track.source_id).ok_or(())?;
                    let capture =
                        start_stream(session_id.clone(), track.track_id.clone(), choice, started)?;
                    writer.state(AudioProviderState::Capturing, "capture.active")?;
                    writer.send(
                        AudioProviderMessage::CaptureStarted {
                            session_id: session_id.clone(),
                            tracks: vec![AudioStartedTrack {
                                track_id: track.track_id.clone(),
                                source_id: track.source_id.clone(),
                                profile: track.profile,
                                provider_start_monotonic_ns: capture.start_monotonic_ns,
                            }],
                            provider_monotonic_ns: capture.start_monotonic_ns,
                        },
                        &[],
                    )?;
                    active = Some(capture);
                }
                AudioHostMessage::DrainCapture {
                    session_id,
                    maximum_frames,
                } => {
                    let capture = active
                        .as_mut()
                        .filter(|capture| capture.session_id == session_id)
                        .ok_or(())?;
                    let dropped = capture.dropped_frames.swap(0, Ordering::AcqRel);
                    if dropped > 0 {
                        writer.send(
                            AudioProviderMessage::CaptureEvent {
                                session_id: session_id.clone(),
                                track_id: Some(capture.track_id.clone()),
                                provider_monotonic_ns: monotonic_ns(started),
                                event: AudioCaptureEventKind::Backpressure,
                                code: "capture.queue_backpressure".into(),
                                affected_frames: Some(dropped.min(48_000 * 60)),
                                drift_parts_per_million: None,
                            },
                            &[],
                        )?;
                    }
                    if capture.stream_failed.swap(false, Ordering::AcqRel) {
                        writer.send(
                            AudioProviderMessage::CaptureEvent {
                                session_id: session_id.clone(),
                                track_id: Some(capture.track_id.clone()),
                                provider_monotonic_ns: monotonic_ns(started),
                                event: AudioCaptureEventKind::SourceUnavailable,
                                code: "capture.stream_failed".into(),
                                affected_frames: None,
                                drift_parts_per_million: None,
                            },
                            &[],
                        )?;
                    }
                    let mut sent = 0_u16;
                    while sent < maximum_frames {
                        let frame = match capture.receiver.try_recv() {
                            Ok(frame) => frame,
                            Err(mpsc::TryRecvError::Empty) => break,
                            Err(mpsc::TryRecvError::Disconnected) => return Err(()),
                        };
                        capture.next_sequence = capture.next_sequence.checked_add(1).ok_or(())?;
                        writer.send(
                            AudioProviderMessage::PcmFrame {
                                session_id: session_id.clone(),
                                track_id: capture.track_id.clone(),
                                frame_sequence: capture.next_sequence,
                                provider_monotonic_ns: frame.provider_monotonic_ns,
                                channels: 1,
                                sample_rate_hz: 48_000,
                                frame_count: FRAME_COUNT as u16,
                                payload_bytes: FRAME_BYTES as u32,
                            },
                            &frame.bytes,
                        )?;
                        writer.send(
                            AudioProviderMessage::Level {
                                session_id: session_id.clone(),
                                track_id: capture.track_id.clone(),
                                provider_monotonic_ns: frame.provider_monotonic_ns,
                                peak_millidbfs: frame.peak_millidbfs,
                                clipped: frame.clipped,
                            },
                            &[],
                        )?;
                        sent += 1;
                    }
                    writer.send(
                        AudioProviderMessage::DrainComplete {
                            session_id,
                            frame_count: sent,
                        },
                        &[],
                    )?;
                }
                AudioHostMessage::StopCapture { session_id } => {
                    let capture = active
                        .take()
                        .filter(|capture| capture.session_id == session_id)
                        .ok_or(())?;
                    drop(capture);
                    writer.send(
                        AudioProviderMessage::CaptureStopped {
                            session_id,
                            provider_monotonic_ns: monotonic_ns(started),
                            reason: AudioStopReason::UserRequested,
                        },
                        &[],
                    )?;
                    writer.state(AudioProviderState::Ready, "provider.ready")?;
                }
                AudioHostMessage::Shutdown => {
                    drop(active.take());
                    writer.state(AudioProviderState::Stopped, "provider.stopped")?;
                    return Ok(());
                }
                AudioHostMessage::Hello { .. } => return Err(()),
            }
        }
    }

    struct DeviceChoice {
        device: Device,
        source_id: String,
        display_name: String,
        config: StreamConfig,
        sample_format: SampleFormat,
    }

    struct CapturedFrame {
        bytes: Vec<u8>,
        provider_monotonic_ns: u64,
        peak_millidbfs: i32,
        clipped: bool,
    }

    struct ActiveCapture {
        session_id: String,
        track_id: String,
        receiver: mpsc::Receiver<CapturedFrame>,
        _stream: Stream,
        dropped_frames: Arc<AtomicU64>,
        stream_failed: Arc<AtomicBool>,
        start_monotonic_ns: u64,
        next_sequence: u64,
    }

    fn enumerate_sources(
        permissions: &BTreeMap<String, AudioPermissionState>,
    ) -> Vec<AudioSourceCapability> {
        device_choices()
            .into_iter()
            .map(|choice| AudioSourceCapability {
                schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
                id: choice.source_id.clone(),
                display_name: choice.display_name,
                role: AudioSourceRole::MicrophoneInput,
                direction: AudioSourceDirection::Input,
                truth: AudioSourceTruth::Isolated,
                availability: AudioSourceAvailability::Available,
                permission: permissions
                    .get(&choice.source_id)
                    .copied()
                    .unwrap_or(AudioPermissionState::PromptRequired),
                channels: 1,
                native_sample_rate_hz: 48_000,
                supported_profiles: vec![AudioProfileId::PilotMicrophoneV1],
                supports_hot_plug: true,
                origin: AudioSourceOrigin::OperatingSystem,
            })
            .collect()
    }

    fn device_choices() -> Vec<DeviceChoice> {
        let host = cpal::default_host();
        let Ok(devices) = host.input_devices() else {
            return Vec::new();
        };
        devices.filter_map(device_choice).collect()
    }

    fn find_device(source_id: &str) -> Option<DeviceChoice> {
        device_choices()
            .into_iter()
            .find(|choice| choice.source_id == source_id)
    }

    fn device_choice(device: Device) -> Option<DeviceChoice> {
        let raw_id = device.id().ok()?.to_string();
        let source_id = stable_source_id(&raw_id);
        let display_name = bounded_display_name(device.description().ok()?.name());
        let config = device
            .supported_input_configs()
            .ok()?
            .filter(|config| {
                config.min_sample_rate() <= 48_000
                    && config.max_sample_rate() >= 48_000
                    && matches!(
                        config.sample_format(),
                        SampleFormat::F32 | SampleFormat::I16
                    )
            })
            .min_by_key(|config| {
                let format_rank = if config.sample_format() == SampleFormat::F32 {
                    0
                } else {
                    1
                };
                (format_rank, config.channels())
            })?;
        let sample_format = config.sample_format();
        let mut config = config.with_sample_rate(48_000).config();
        config.buffer_size = BufferSize::Fixed(FRAME_COUNT as u32);
        Some(DeviceChoice {
            device,
            source_id,
            display_name,
            config,
            sample_format,
        })
    }

    fn probe_permission(choice: &DeviceChoice) -> Result<(), ()> {
        let stream = match choice.sample_format {
            SampleFormat::F32 => choice.device.build_input_stream::<f32, _, _>(
                choice.config,
                |_, _| {},
                |_| {},
                Some(STREAM_OPEN_TIMEOUT),
            ),
            SampleFormat::I16 => choice.device.build_input_stream::<i16, _, _>(
                choice.config,
                |_, _| {},
                |_| {},
                Some(STREAM_OPEN_TIMEOUT),
            ),
            _ => return Err(()),
        }
        .map_err(|_| ())?;
        drop(stream);
        Ok(())
    }

    fn start_stream(
        session_id: String,
        track_id: String,
        choice: DeviceChoice,
        provider_started: Instant,
    ) -> Result<ActiveCapture, ()> {
        let (sender, receiver) = mpsc::sync_channel(CAPTURE_QUEUE_FRAMES);
        let dropped_frames = Arc::new(AtomicU64::new(0));
        let stream_failed = Arc::new(AtomicBool::new(false));
        let failed_for_callback = Arc::clone(&stream_failed);
        let channels = usize::from(choice.config.channels);
        let stream = match choice.sample_format {
            SampleFormat::F32 => {
                let dropped = Arc::clone(&dropped_frames);
                let mut accumulator = Vec::with_capacity(FRAME_COUNT * 2);
                choice.device.build_input_stream::<f32, _, _>(
                    choice.config,
                    move |samples, _| {
                        append_f32(samples, channels, &mut accumulator);
                        send_complete_frames(&mut accumulator, &sender, &dropped, provider_started);
                    },
                    move |_| failed_for_callback.store(true, Ordering::Release),
                    Some(STREAM_OPEN_TIMEOUT),
                )
            }
            SampleFormat::I16 => {
                let dropped = Arc::clone(&dropped_frames);
                let mut accumulator = Vec::with_capacity(FRAME_COUNT * 2);
                choice.device.build_input_stream::<i16, _, _>(
                    choice.config,
                    move |samples, _| {
                        append_i16(samples, channels, &mut accumulator);
                        send_complete_frames(&mut accumulator, &sender, &dropped, provider_started);
                    },
                    move |_| failed_for_callback.store(true, Ordering::Release),
                    Some(STREAM_OPEN_TIMEOUT),
                )
            }
            _ => return Err(()),
        }
        .map_err(|_| ())?;
        stream.play().map_err(|_| ())?;
        let start_monotonic_ns = monotonic_ns(provider_started);
        Ok(ActiveCapture {
            session_id,
            track_id,
            receiver,
            _stream: stream,
            dropped_frames,
            stream_failed,
            start_monotonic_ns,
            next_sequence: 0,
        })
    }

    fn append_f32(samples: &[f32], channels: usize, accumulator: &mut Vec<i16>) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_exact(channels) {
            let average = frame.iter().copied().sum::<f32>() / channels as f32;
            accumulator.push(float_to_i16(average));
        }
    }

    fn append_i16(samples: &[i16], channels: usize, accumulator: &mut Vec<i16>) {
        if channels == 0 {
            return;
        }
        for frame in samples.chunks_exact(channels) {
            let total = frame.iter().map(|sample| i64::from(*sample)).sum::<i64>();
            accumulator.push((total / channels as i64) as i16);
        }
    }

    fn send_complete_frames(
        accumulator: &mut Vec<i16>,
        sender: &mpsc::SyncSender<CapturedFrame>,
        dropped_frames: &AtomicU64,
        provider_started: Instant,
    ) {
        while accumulator.len() >= FRAME_COUNT {
            let samples = accumulator.drain(..FRAME_COUNT).collect::<Vec<_>>();
            let (peak_millidbfs, clipped) = level(&samples);
            let bytes = samples
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect();
            if sender
                .try_send(CapturedFrame {
                    bytes,
                    provider_monotonic_ns: monotonic_ns(provider_started),
                    peak_millidbfs,
                    clipped,
                })
                .is_err()
            {
                dropped_frames.fetch_add(FRAME_COUNT as u64, Ordering::Relaxed);
            }
        }
    }

    fn float_to_i16(sample: f32) -> i16 {
        (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
    }

    fn level(samples: &[i16]) -> (i32, bool) {
        let peak = samples
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap_or(0);
        let clipped = peak >= i16::MAX as u16;
        if peak == 0 {
            (-120_000, false)
        } else {
            let normalized = f64::from(peak) / f64::from(i16::MAX);
            let millidbfs = (20_000.0 * normalized.log10()).round() as i32;
            (millidbfs.clamp(-120_000, 0), clipped)
        }
    }

    fn stable_source_id(raw_id: &str) -> String {
        let digest = Sha256::digest(raw_id.as_bytes());
        let digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("windows-microphone:{digest}")
    }

    fn bounded_display_name(name: &str) -> String {
        let sanitized = name
            .chars()
            .filter(|character| !character.is_control())
            .take(120)
            .collect::<String>();
        if sanitized.trim().is_empty() {
            "Windows microphone".into()
        } else {
            sanitized
        }
    }

    fn monotonic_ns(started: Instant) -> u64 {
        u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn descriptor() -> AudioProviderDescriptor {
        AudioProviderDescriptor {
            id: PROVIDER_ID.into(),
            name: "WyrmGrid Windows microphone provider".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: AudioProviderPlatform::WindowsX86_64,
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

    struct ProviderWriter {
        stdout: BufWriter<Stdout>,
        sequence: u64,
    }

    impl ProviderWriter {
        fn new(stdout: Stdout) -> Self {
            Self {
                stdout: BufWriter::new(stdout),
                sequence: 0,
            }
        }

        fn send(&mut self, message: AudioProviderMessage, body: &[u8]) -> Result<(), ()> {
            self.sequence = self.sequence.checked_add(1).ok_or(())?;
            write_provider_frame(
                &mut self.stdout,
                &AudioEnvelope::new(self.sequence, message),
                body,
            )
            .map_err(|_| ())
        }

        fn state(&mut self, state: AudioProviderState, code: &str) -> Result<(), ()> {
            self.send(
                AudioProviderMessage::State {
                    state,
                    code: code.into(),
                },
                &[],
            )
        }
    }

    #[cfg(test)]
    #[path = "tests/unit.rs"]
    mod tests;
}

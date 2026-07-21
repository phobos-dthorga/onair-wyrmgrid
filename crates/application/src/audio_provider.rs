use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_audio_provider_protocol::{
    AudioCaptureEventKind, AudioEnvelope, AudioHostMessage, AudioProviderDescriptor,
    AudioProviderManifest, AudioProviderMessage, AudioProviderPlatform, AudioProviderState,
    AudioStopReason, AudioTrackRequest, read_provider_frame, validate_next_sequence,
    write_host_frame,
};
use wyrmgrid_domain::{AudioSourceCapability, AudioSourceTruth};

use crate::EncodedAudioPacket;

const STARTUP_MESSAGE_COUNT: usize = 3;
const FAKE_CAPTURE_MESSAGE_COUNT: usize = 5;
const PROVIDER_IO_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct AudioProviderRegistration {
    pub manifest: AudioProviderManifest,
    pub executable: PathBuf,
}

impl AudioProviderRegistration {
    pub fn from_manifest_json(
        manifest_json: &str,
        executable: impl Into<PathBuf>,
    ) -> Result<Self, AudioProviderError> {
        let manifest: AudioProviderManifest =
            serde_json::from_str(manifest_json).map_err(|_| AudioProviderError::InvalidManifest)?;
        manifest
            .validate()
            .map_err(|_| AudioProviderError::InvalidManifest)?;
        Ok(Self {
            manifest,
            executable: executable.into(),
        })
    }

    pub(crate) fn from_managed_package(
        manifest: AudioProviderManifest,
        root: PathBuf,
    ) -> Result<Self, AudioProviderError> {
        manifest
            .validate()
            .map_err(|_| AudioProviderError::InvalidManifest)?;
        let executable = provider_executable_in(&root, &manifest);
        Ok(Self {
            manifest,
            executable,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProviderPacket {
    pub session_id: String,
    pub track_id: String,
    pub packet: EncodedAudioPacket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProviderLevel {
    pub session_id: String,
    pub track_id: String,
    pub provider_monotonic_ns: u64,
    pub peak_millidbfs: i32,
    pub clipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProviderEvent {
    pub session_id: String,
    pub track_id: Option<String>,
    pub provider_monotonic_ns: u64,
    pub event: AudioCaptureEventKind,
    pub code: String,
    pub affected_frames: Option<u64>,
    pub drift_parts_per_million: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProviderCaptureBatch {
    pub provider_start_monotonic_ns: u64,
    pub tracks: Vec<wyrmgrid_audio_provider_protocol::AudioStartedTrack>,
    pub packets: Vec<AudioProviderPacket>,
    pub levels: Vec<AudioProviderLevel>,
    pub events: Vec<AudioProviderEvent>,
}

pub trait AudioCaptureProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn sources(&self) -> Result<Vec<AudioSourceCapability>, AudioProviderError>;
    fn request_permission(
        &self,
        source_id: &str,
    ) -> Result<Vec<AudioSourceCapability>, AudioProviderError>;
    fn start_capture(
        &self,
        session_id: &str,
        tracks: &[AudioTrackRequest],
    ) -> Result<AudioProviderCaptureBatch, AudioProviderError>;
    fn stop_capture(&self, session_id: &str) -> Result<AudioStopReason, AudioProviderError>;
}

#[derive(Debug, Error)]
pub enum AudioProviderError {
    #[error("the audio provider manifest is invalid")]
    InvalidManifest,
    #[error("the audio provider is unavailable on this platform")]
    Unavailable,
    #[error("the audio provider process could not be started")]
    StartFailed,
    #[error("the audio provider protocol failed or produced an unexpected message")]
    Protocol,
    #[error("the audio provider state is unavailable")]
    StateUnavailable,
    #[error("the audio provider source is unavailable")]
    SourceUnavailable,
}

pub struct ExternalAudioProviderProcess {
    registration: AudioProviderRegistration,
    runtime: Mutex<Option<AudioProcessRuntime>>,
}

struct AudioProcessRuntime {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    outgoing_sequence: u64,
    incoming_sequence: u64,
    active_session_id: Option<String>,
}

impl ExternalAudioProviderProcess {
    pub fn new(registration: AudioProviderRegistration) -> Result<Self, AudioProviderError> {
        if !platform_supported(&registration.manifest) || !registration.executable.is_file() {
            return Err(AudioProviderError::Unavailable);
        }
        Ok(Self {
            registration,
            runtime: Mutex::new(None),
        })
    }

    fn with_runtime<T>(
        &self,
        operation: impl FnOnce(&mut AudioProcessRuntime) -> Result<T, AudioProviderError>,
    ) -> Result<T, AudioProviderError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AudioProviderError::StateUnavailable)?;
        if runtime.is_none() {
            *runtime = Some(launch(&self.registration)?);
        }
        let result = operation(runtime.as_mut().expect("runtime initialized"));
        if result.is_err() {
            *runtime = None;
        }
        result
    }
}

impl AudioCaptureProvider for ExternalAudioProviderProcess {
    fn provider_id(&self) -> &str {
        &self.registration.manifest.id
    }

    fn sources(&self) -> Result<Vec<AudioSourceCapability>, AudioProviderError> {
        self.with_runtime(|runtime| {
            runtime.send(AudioHostMessage::EnumerateSources)?;
            match runtime.receive()?.0.payload {
                AudioProviderMessage::Sources { sources, .. } => Ok(sources),
                _ => Err(AudioProviderError::Protocol),
            }
        })
    }

    fn request_permission(
        &self,
        source_id: &str,
    ) -> Result<Vec<AudioSourceCapability>, AudioProviderError> {
        self.with_runtime(|runtime| {
            runtime.send(AudioHostMessage::RequestPermission {
                source_id: source_id.into(),
            })?;
            match runtime.receive()?.0.payload {
                AudioProviderMessage::Sources { sources, .. } => Ok(sources),
                _ => Err(AudioProviderError::Protocol),
            }
        })
    }

    fn start_capture(
        &self,
        session_id: &str,
        tracks: &[AudioTrackRequest],
    ) -> Result<AudioProviderCaptureBatch, AudioProviderError> {
        self.with_runtime(|runtime| {
            if runtime.active_session_id.is_some() {
                return Err(AudioProviderError::StateUnavailable);
            }
            runtime.send(AudioHostMessage::StartCapture {
                session_id: session_id.into(),
                tracks: tracks.to_vec(),
            })?;
            let mut batch = AudioProviderCaptureBatch {
                provider_start_monotonic_ns: 0,
                tracks: Vec::new(),
                packets: Vec::new(),
                levels: Vec::new(),
                events: Vec::new(),
            };
            for _ in 0..FAKE_CAPTURE_MESSAGE_COUNT {
                let (envelope, body) = runtime.receive()?;
                match envelope.payload {
                    AudioProviderMessage::State {
                        state: AudioProviderState::Capturing,
                        ..
                    } => {}
                    AudioProviderMessage::CaptureStarted {
                        session_id: received,
                        tracks,
                        provider_monotonic_ns,
                    } if received == session_id => {
                        batch.provider_start_monotonic_ns = provider_monotonic_ns;
                        batch.tracks = tracks;
                    }
                    AudioProviderMessage::AudioPacket {
                        session_id: received,
                        track_id,
                        packet_sequence,
                        provider_monotonic_ns,
                        duration_48khz_frames,
                        ..
                    } if received == session_id => batch.packets.push(AudioProviderPacket {
                        session_id: received,
                        track_id,
                        packet: EncodedAudioPacket {
                            sequence: packet_sequence,
                            provider_monotonic_ns,
                            duration_48khz_frames,
                            bytes: body,
                        },
                    }),
                    AudioProviderMessage::Level {
                        session_id: received,
                        track_id,
                        provider_monotonic_ns,
                        peak_millidbfs,
                        clipped,
                    } if received == session_id => batch.levels.push(AudioProviderLevel {
                        session_id: received,
                        track_id,
                        provider_monotonic_ns,
                        peak_millidbfs,
                        clipped,
                    }),
                    AudioProviderMessage::CaptureEvent {
                        session_id: received,
                        track_id,
                        provider_monotonic_ns,
                        event,
                        code,
                        affected_frames,
                        drift_parts_per_million,
                    } if received == session_id => batch.events.push(AudioProviderEvent {
                        session_id: received,
                        track_id,
                        provider_monotonic_ns,
                        event,
                        code,
                        affected_frames,
                        drift_parts_per_million,
                    }),
                    _ => return Err(AudioProviderError::Protocol),
                }
            }
            if batch.tracks.is_empty()
                || batch.packets.is_empty()
                || batch.provider_start_monotonic_ns == 0
            {
                return Err(AudioProviderError::Protocol);
            }
            runtime.active_session_id = Some(session_id.into());
            Ok(batch)
        })
    }

    fn stop_capture(&self, session_id: &str) -> Result<AudioStopReason, AudioProviderError> {
        self.with_runtime(|runtime| {
            if runtime.active_session_id.as_deref() != Some(session_id) {
                return Err(AudioProviderError::StateUnavailable);
            }
            runtime.send(AudioHostMessage::StopCapture {
                session_id: session_id.into(),
            })?;
            let stopped = runtime.receive()?.0.payload;
            let ready = runtime.receive()?.0.payload;
            let reason = match stopped {
                AudioProviderMessage::CaptureStopped {
                    session_id: received,
                    reason,
                    ..
                } if received == session_id => reason,
                _ => return Err(AudioProviderError::Protocol),
            };
            if !matches!(
                ready,
                AudioProviderMessage::State {
                    state: AudioProviderState::Ready,
                    ..
                }
            ) {
                return Err(AudioProviderError::Protocol);
            }
            runtime.active_session_id = None;
            Ok(reason)
        })
    }
}

impl AudioProcessRuntime {
    fn send(&mut self, message: AudioHostMessage) -> Result<(), AudioProviderError> {
        self.outgoing_sequence = self
            .outgoing_sequence
            .checked_add(1)
            .ok_or(AudioProviderError::Protocol)?;
        write_host_frame(
            &mut self.stdin,
            &AudioEnvelope::new(self.outgoing_sequence, message),
        )
        .map_err(|_| AudioProviderError::Protocol)
    }

    fn receive(
        &mut self,
    ) -> Result<(AudioEnvelope<AudioProviderMessage>, Vec<u8>), AudioProviderError> {
        let received = thread::scope(|scope| {
            let (sender, receiver) = mpsc::sync_channel(1);
            let stdout = &mut self.stdout;
            scope.spawn(move || {
                let _ = sender.send(read_provider_frame(stdout));
            });
            match receiver.recv_timeout(PROVIDER_IO_TIMEOUT) {
                Ok(Ok(received)) => Ok(received),
                Ok(Err(_)) => Err(AudioProviderError::Protocol),
                Err(_) => {
                    let _ = self.child.kill();
                    Err(AudioProviderError::Protocol)
                }
            }
        })?;
        if !validate_next_sequence(self.incoming_sequence, received.0.sequence) {
            return Err(AudioProviderError::Protocol);
        }
        self.incoming_sequence = received.0.sequence;
        Ok(received)
    }
}

impl Drop for AudioProcessRuntime {
    fn drop(&mut self) {
        let _ = self.send(AudioHostMessage::Shutdown);
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() < PROVIDER_IO_TIMEOUT => {
                    thread::sleep(Duration::from_millis(20));
                }
                Ok(None) | Err(_) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
            }
        }
    }
}

fn launch(
    registration: &AudioProviderRegistration,
) -> Result<AudioProcessRuntime, AudioProviderError> {
    let executable =
        fs::canonicalize(&registration.executable).map_err(|_| AudioProviderError::Unavailable)?;
    let mut child = Command::new(executable)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AudioProviderError::StartFailed)?;
    let stdin = child.stdin.take().ok_or(AudioProviderError::StartFailed)?;
    let stdout = child.stdout.take().ok_or(AudioProviderError::StartFailed)?;
    let mut runtime = AudioProcessRuntime {
        child,
        stdin: BufWriter::new(stdin),
        stdout: BufReader::new(stdout),
        outgoing_sequence: 0,
        incoming_sequence: 0,
        active_session_id: None,
    };
    runtime.send(AudioHostMessage::Hello {
        host_version: env!("CARGO_PKG_VERSION").into(),
        provider_id: registration.manifest.id.clone(),
    })?;
    let mut hello_seen = false;
    let mut ready_seen = false;
    for _ in 0..STARTUP_MESSAGE_COUNT {
        match runtime.receive()?.0.payload {
            AudioProviderMessage::Hello { provider }
                if provider_matches_manifest(&provider, &registration.manifest) =>
            {
                hello_seen = true
            }
            AudioProviderMessage::State {
                state: AudioProviderState::Starting,
                ..
            } => {}
            AudioProviderMessage::State {
                state: AudioProviderState::Ready,
                ..
            } => ready_seen = true,
            _ => return Err(AudioProviderError::Protocol),
        }
    }
    if !hello_seen || !ready_seen {
        return Err(AudioProviderError::Protocol);
    }
    Ok(runtime)
}

fn provider_matches_manifest(
    provider: &AudioProviderDescriptor,
    manifest: &AudioProviderManifest,
) -> bool {
    provider.validate()
        && provider.id == manifest.id
        && provider.name == manifest.name
        && provider.version == manifest.version
        && provider.platform == current_audio_provider_platform()
        && provider.capabilities.len() == manifest.capabilities.len()
        && provider
            .capabilities
            .iter()
            .all(|capability| manifest.capabilities.contains(capability))
}

fn platform_supported(manifest: &AudioProviderManifest) -> bool {
    let platform = current_audio_provider_platform();
    manifest.platforms.contains(&platform)
}

pub(crate) fn current_audio_provider_platform() -> AudioProviderPlatform {
    #[cfg(target_os = "windows")]
    return AudioProviderPlatform::WindowsX86_64;
    #[cfg(target_os = "linux")]
    return AudioProviderPlatform::LinuxX86_64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return AudioProviderPlatform::MacosAarch64;
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    return AudioProviderPlatform::MacosX86_64;
}

pub fn source_truth_id(truth: AudioSourceTruth) -> &'static str {
    match truth {
        AudioSourceTruth::Isolated => "isolated",
        AudioSourceTruth::MixedOutput => "mixed_output",
        AudioSourceTruth::MetadataOnly => "metadata_only",
    }
}

pub fn provider_executable_in(directory: &Path, manifest: &AudioProviderManifest) -> PathBuf {
    let mut filename = manifest.entry_point.clone();
    if cfg!(windows) {
        filename.push_str(".exe");
    }
    directory.join(filename)
}

#[cfg(test)]
#[path = "tests/audio_provider.rs"]
mod tests;

use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_audio_codec_protocol::{
    AudioCodecHostMessage, AudioCodecManifest, AudioCodecPlatform, AudioCodecProfile,
    AudioCodecProviderMessage, AudioCodecState, CodecEnvelope, PcmSampleFormat,
    read_codec_provider_frame, validate_next_codec_sequence, write_codec_host_frame,
};
use wyrmgrid_domain::AudioProfileId;

use crate::{AudioProviderPcmFrame, EncodedAudioPacket};

const STARTUP_MESSAGE_COUNT: usize = 3;
const CODEC_IO_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct AudioCodecRegistration {
    pub manifest: AudioCodecManifest,
    pub executable: PathBuf,
}

impl AudioCodecRegistration {
    pub fn from_manifest_json(
        manifest_json: &str,
        executable: impl Into<PathBuf>,
    ) -> Result<Self, AudioCodecError> {
        let manifest: AudioCodecManifest =
            serde_json::from_str(manifest_json).map_err(|_| AudioCodecError::InvalidManifest)?;
        manifest
            .validate()
            .map_err(|_| AudioCodecError::InvalidManifest)?;
        Ok(Self {
            manifest,
            executable: executable.into(),
        })
    }

    pub(crate) fn from_managed_package(
        manifest: AudioCodecManifest,
        root: PathBuf,
    ) -> Result<Self, AudioCodecError> {
        manifest
            .validate()
            .map_err(|_| AudioCodecError::InvalidManifest)?;
        let executable = codec_executable_in(&root, &manifest);
        let canonical_root = root
            .canonicalize()
            .map_err(|_| AudioCodecError::Unavailable)?;
        let canonical_executable = executable
            .canonicalize()
            .map_err(|_| AudioCodecError::Unavailable)?;
        if !canonical_executable.starts_with(&canonical_root) || !canonical_executable.is_file() {
            return Err(AudioCodecError::Unavailable);
        }
        Ok(Self {
            manifest,
            executable: canonical_executable,
        })
    }
}

pub trait AudioCodecProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn provider_version(&self) -> &str;
    fn display_name(&self) -> &str;
    fn profiles(&self) -> &[AudioCodecProfile];
    fn start_track(
        &self,
        session_id: &str,
        track_id: &str,
        profile: AudioProfileId,
    ) -> Result<(), AudioCodecError>;
    fn encode_pcm(
        &self,
        frame: &AudioProviderPcmFrame,
    ) -> Result<EncodedAudioPacket, AudioCodecError>;
    fn stop_track(&self, session_id: &str, track_id: &str) -> Result<(), AudioCodecError>;
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecError {
    #[error("the audio codec manifest is invalid")]
    InvalidManifest,
    #[error("the audio codec is unavailable on this platform")]
    Unavailable,
    #[error("the audio codec process could not be started")]
    StartFailed,
    #[error("the audio codec protocol failed or produced an unexpected message")]
    Protocol,
    #[error("the audio codec state is unavailable")]
    StateUnavailable,
    #[error("the selected audio profile is unsupported by this codec")]
    UnsupportedProfile,
}

pub struct ProcessAudioCodecProvider {
    registration: AudioCodecRegistration,
    runtime: Mutex<Option<AudioCodecProcessRuntime>>,
}

struct AudioCodecProcessRuntime {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    outgoing_sequence: u64,
    incoming_sequence: u64,
}

impl ProcessAudioCodecProvider {
    pub fn new(registration: AudioCodecRegistration) -> Result<Self, AudioCodecError> {
        if !codec_platform_supported(&registration.manifest) || !registration.executable.is_file() {
            return Err(AudioCodecError::Unavailable);
        }
        Ok(Self {
            registration,
            runtime: Mutex::new(None),
        })
    }

    fn with_runtime<T>(
        &self,
        operation: impl FnOnce(&mut AudioCodecProcessRuntime) -> Result<T, AudioCodecError>,
    ) -> Result<T, AudioCodecError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AudioCodecError::StateUnavailable)?;
        if runtime.is_none() {
            *runtime = Some(launch_codec(&self.registration)?);
        }
        let result = operation(runtime.as_mut().expect("runtime initialized"));
        if result.is_err() {
            *runtime = None;
        }
        result
    }
}

impl AudioCodecProvider for ProcessAudioCodecProvider {
    fn provider_id(&self) -> &str {
        &self.registration.manifest.id
    }

    fn provider_version(&self) -> &str {
        &self.registration.manifest.version
    }

    fn display_name(&self) -> &str {
        &self.registration.manifest.name
    }

    fn profiles(&self) -> &[AudioCodecProfile] {
        &self.registration.manifest.profiles
    }

    fn start_track(
        &self,
        session_id: &str,
        track_id: &str,
        profile: AudioProfileId,
    ) -> Result<(), AudioCodecError> {
        if !self
            .registration
            .manifest
            .profiles
            .iter()
            .any(|candidate| candidate.id == profile)
        {
            return Err(AudioCodecError::UnsupportedProfile);
        }
        self.with_runtime(|runtime| {
            runtime.send(
                AudioCodecHostMessage::StartTrack {
                    session_id: session_id.into(),
                    track_id: track_id.into(),
                    profile,
                },
                &[],
            )?;
            match runtime.receive()?.0.payload {
                AudioCodecProviderMessage::TrackStarted {
                    session_id: received_session,
                    track_id: received_track,
                    profile: received_profile,
                } if received_session == session_id
                    && received_track == track_id
                    && received_profile == profile =>
                {
                    Ok(())
                }
                _ => Err(AudioCodecError::Protocol),
            }
        })
    }

    fn encode_pcm(
        &self,
        frame: &AudioProviderPcmFrame,
    ) -> Result<EncodedAudioPacket, AudioCodecError> {
        self.with_runtime(|runtime| {
            runtime.send(
                AudioCodecHostMessage::EncodePcm {
                    session_id: frame.session_id.clone(),
                    track_id: frame.track_id.clone(),
                    frame_sequence: frame.frame_sequence,
                    provider_monotonic_ns: frame.provider_monotonic_ns,
                    sample_format: PcmSampleFormat::S16le,
                    channels: frame.channels,
                    sample_rate_hz: frame.sample_rate_hz,
                    frame_count: frame.frame_count,
                    payload_bytes: u32::try_from(frame.bytes.len())
                        .map_err(|_| AudioCodecError::Protocol)?,
                },
                &frame.bytes,
            )?;
            let (envelope, body) = runtime.receive()?;
            match envelope.payload {
                AudioCodecProviderMessage::EncodedPacket {
                    session_id,
                    track_id,
                    packet_sequence,
                    provider_monotonic_ns,
                    duration_48khz_frames,
                    ..
                } if session_id == frame.session_id
                    && track_id == frame.track_id
                    && packet_sequence == frame.frame_sequence
                    && provider_monotonic_ns == frame.provider_monotonic_ns
                    && duration_48khz_frames == frame.frame_count =>
                {
                    Ok(EncodedAudioPacket {
                        sequence: packet_sequence,
                        provider_monotonic_ns,
                        duration_48khz_frames,
                        bytes: body,
                    })
                }
                _ => Err(AudioCodecError::Protocol),
            }
        })
    }

    fn stop_track(&self, session_id: &str, track_id: &str) -> Result<(), AudioCodecError> {
        self.with_runtime(|runtime| {
            runtime.send(
                AudioCodecHostMessage::StopTrack {
                    session_id: session_id.into(),
                    track_id: track_id.into(),
                },
                &[],
            )?;
            match runtime.receive()?.0.payload {
                AudioCodecProviderMessage::TrackStopped {
                    session_id: received_session,
                    track_id: received_track,
                } if received_session == session_id && received_track == track_id => Ok(()),
                _ => Err(AudioCodecError::Protocol),
            }
        })
    }
}

impl AudioCodecProcessRuntime {
    fn send(&mut self, message: AudioCodecHostMessage, body: &[u8]) -> Result<(), AudioCodecError> {
        self.outgoing_sequence = self
            .outgoing_sequence
            .checked_add(1)
            .ok_or(AudioCodecError::Protocol)?;
        write_codec_host_frame(
            &mut self.stdin,
            &CodecEnvelope::new(self.outgoing_sequence, message),
            body,
        )
        .map_err(|_| AudioCodecError::Protocol)
    }

    fn receive(
        &mut self,
    ) -> Result<(CodecEnvelope<AudioCodecProviderMessage>, Vec<u8>), AudioCodecError> {
        let received = thread::scope(|scope| {
            let (sender, receiver) = mpsc::sync_channel(1);
            let stdout = &mut self.stdout;
            scope.spawn(move || {
                let _ = sender.send(read_codec_provider_frame(stdout));
            });
            match receiver.recv_timeout(CODEC_IO_TIMEOUT) {
                Ok(Ok(received)) => Ok(received),
                Ok(Err(_)) => Err(AudioCodecError::Protocol),
                Err(_) => {
                    let _ = self.child.kill();
                    Err(AudioCodecError::Protocol)
                }
            }
        })?;
        if !validate_next_codec_sequence(self.incoming_sequence, received.0.sequence) {
            return Err(AudioCodecError::Protocol);
        }
        self.incoming_sequence = received.0.sequence;
        Ok(received)
    }
}

impl Drop for AudioCodecProcessRuntime {
    fn drop(&mut self) {
        let _ = self.send(AudioCodecHostMessage::Shutdown, &[]);
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() < CODEC_IO_TIMEOUT => {
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

fn launch_codec(
    registration: &AudioCodecRegistration,
) -> Result<AudioCodecProcessRuntime, AudioCodecError> {
    let executable =
        fs::canonicalize(&registration.executable).map_err(|_| AudioCodecError::Unavailable)?;
    let mut child = Command::new(executable)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AudioCodecError::StartFailed)?;
    let stdin = child.stdin.take().ok_or(AudioCodecError::StartFailed)?;
    let stdout = child.stdout.take().ok_or(AudioCodecError::StartFailed)?;
    let mut runtime = AudioCodecProcessRuntime {
        child,
        stdin: BufWriter::new(stdin),
        stdout: BufReader::new(stdout),
        outgoing_sequence: 0,
        incoming_sequence: 0,
    };
    runtime.send(
        AudioCodecHostMessage::Hello {
            host_version: env!("CARGO_PKG_VERSION").into(),
            codec_provider_id: registration.manifest.id.clone(),
        },
        &[],
    )?;
    let mut hello_seen = false;
    let mut ready_seen = false;
    for _ in 0..STARTUP_MESSAGE_COUNT {
        match runtime.receive()?.0.payload {
            AudioCodecProviderMessage::Hello { codec }
                if codec.id == registration.manifest.id
                    && codec.name == registration.manifest.name
                    && codec.version == registration.manifest.version
                    && codec.platform == current_codec_platform()
                    && codec.profiles == registration.manifest.profiles =>
            {
                hello_seen = true;
            }
            AudioCodecProviderMessage::State {
                state: AudioCodecState::Starting,
                ..
            } => {}
            AudioCodecProviderMessage::State {
                state: AudioCodecState::Ready,
                ..
            } => ready_seen = true,
            _ => return Err(AudioCodecError::Protocol),
        }
    }
    if !hello_seen || !ready_seen {
        return Err(AudioCodecError::Protocol);
    }
    Ok(runtime)
}

fn codec_platform_supported(manifest: &AudioCodecManifest) -> bool {
    manifest.platforms.contains(&current_codec_platform())
}

pub(crate) fn current_codec_platform() -> AudioCodecPlatform {
    #[cfg(target_os = "windows")]
    return AudioCodecPlatform::WindowsX86_64;
    #[cfg(target_os = "linux")]
    return AudioCodecPlatform::LinuxX86_64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return AudioCodecPlatform::MacosAarch64;
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    return AudioCodecPlatform::MacosX86_64;
}

pub fn codec_executable_in(directory: &Path, manifest: &AudioCodecManifest) -> PathBuf {
    let mut filename = manifest.entry_point.clone();
    if cfg!(windows) {
        filename.push_str(".exe");
    }
    directory.join(filename)
}

#[cfg(test)]
#[path = "tests/audio_codec.rs"]
mod tests;

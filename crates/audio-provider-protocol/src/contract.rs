use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};
use thiserror::Error;
use wyrmgrid_domain::{AUDIO_WORKING_SAMPLE_RATE_HZ, AudioProfileId, AudioSourceCapability};

pub const AUDIO_PROVIDER_PROTOCOL_VERSION: u32 = 2;
pub const AUDIO_PROVIDER_MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const MAX_AUDIO_CONTROL_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_PCM_AUDIO_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_AUDIO_SOURCES: usize = 32;
pub const MAX_AUDIO_TRACKS: usize = 8;
pub const MAX_AUDIO_PROVIDER_CAPABILITIES: usize = 6;
pub const MAX_AUDIO_DRAIN_FRAMES: u16 = 64;
pub const MAX_AUDIO_EVENT_FRAMES: u64 = 48_000 * 60;
pub const MAX_ABSOLUTE_AUDIO_DRIFT_PPM: i32 = 100_000;
pub const MIN_AUDIO_LEVEL_MILLIDBFS: i32 = -120_000;
pub const MAX_AUDIO_LEVEL_MILLIDBFS: i32 = 0;
pub const PCM_FRAME_DURATIONS_48KHZ: [u16; 6] = [120, 240, 480, 960, 1_920, 2_880];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProviderPlatform {
    WindowsX86_64,
    LinuxX86_64,
    MacosAarch64,
    MacosX86_64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProviderCapability {
    SourceEnumeration,
    PermissionRequests,
    PcmS16leCapture,
    LevelMetering,
    HotPlugNotifications,
    ClockSynchronization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioProviderManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub audio_protocol_version: u32,
    pub author: String,
    pub entry_point: String,
    pub platforms: Vec<AudioProviderPlatform>,
    pub capabilities: Vec<AudioProviderCapability>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioProviderManifestError {
    #[error("unsupported audio provider manifest schema")]
    UnsupportedSchema,
    #[error("audio provider id must use reverse-domain notation")]
    InvalidId,
    #[error("audio provider metadata is invalid")]
    InvalidMetadata,
    #[error("audio provider uses an unsupported protocol")]
    UnsupportedProtocol,
    #[error("audio provider entry point must be a safe relative path")]
    UnsafeEntryPoint,
    #[error("audio provider must declare unique platforms and capabilities")]
    InvalidDeclaration,
}

impl AudioProviderManifest {
    pub fn validate(&self) -> Result<(), AudioProviderManifestError> {
        if self.schema_version != AUDIO_PROVIDER_MANIFEST_SCHEMA_VERSION {
            return Err(AudioProviderManifestError::UnsupportedSchema);
        }
        if self
            .schema
            .as_ref()
            .is_some_and(|schema| !valid_text(schema, 256))
        {
            return Err(AudioProviderManifestError::InvalidMetadata);
        }
        if !valid_reverse_domain_id(&self.id) {
            return Err(AudioProviderManifestError::InvalidId);
        }
        if !valid_text(&self.name, 120)
            || !valid_text(&self.author, 120)
            || !valid_semantic_version(&self.version)
        {
            return Err(AudioProviderManifestError::InvalidMetadata);
        }
        if self.audio_protocol_version != AUDIO_PROVIDER_PROTOCOL_VERSION {
            return Err(AudioProviderManifestError::UnsupportedProtocol);
        }
        let entry_point = Path::new(&self.entry_point);
        if self.entry_point.trim().is_empty()
            || entry_point.is_absolute()
            || entry_point.components().any(|component| {
                matches!(
                    component,
                    Component::CurDir
                        | Component::ParentDir
                        | Component::RootDir
                        | Component::Prefix(_)
                )
            })
        {
            return Err(AudioProviderManifestError::UnsafeEntryPoint);
        }
        if self.platforms.is_empty()
            || self.capabilities.is_empty()
            || self.capabilities.len() > MAX_AUDIO_PROVIDER_CAPABILITIES
            || unique_count(&self.platforms) != self.platforms.len()
            || unique_count(&self.capabilities) != self.capabilities.len()
        {
            return Err(AudioProviderManifestError::InvalidDeclaration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioEnvelope<T> {
    pub protocol_version: u32,
    pub sequence: u64,
    pub payload: T,
}

impl<T> AudioEnvelope<T> {
    pub fn new(sequence: u64, payload: T) -> Self {
        Self {
            protocol_version: AUDIO_PROVIDER_PROTOCOL_VERSION,
            sequence,
            payload,
        }
    }

    pub fn validate_header(&self) -> Result<(), AudioEnvelopeError> {
        if self.protocol_version != AUDIO_PROVIDER_PROTOCOL_VERSION {
            return Err(AudioEnvelopeError::UnsupportedProtocolVersion);
        }
        if self.sequence == 0 {
            return Err(AudioEnvelopeError::InvalidSequence);
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioEnvelopeError {
    #[error("unsupported audio provider protocol version")]
    UnsupportedProtocolVersion,
    #[error("audio provider message sequence must be greater than zero")]
    InvalidSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioTrackRequest {
    pub track_id: String,
    pub source_id: String,
    pub profile: AudioProfileId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioHostMessage {
    Hello {
        host_version: String,
        provider_id: String,
    },
    EnumerateSources,
    RequestPermission {
        source_id: String,
    },
    SynchronizeClock {
        request_id: u64,
        host_send_monotonic_ns: u64,
    },
    StartCapture {
        session_id: String,
        tracks: Vec<AudioTrackRequest>,
    },
    DrainCapture {
        session_id: String,
        maximum_frames: u16,
    },
    StopCapture {
        session_id: String,
    },
    Shutdown,
}

impl AudioHostMessage {
    pub fn validate(&self) -> Result<(), AudioMessageError> {
        match self {
            Self::Hello {
                host_version,
                provider_id,
            } if valid_semantic_version(host_version) && valid_reverse_domain_id(provider_id) => {
                Ok(())
            }
            Self::EnumerateSources | Self::Shutdown => Ok(()),
            Self::RequestPermission { source_id } if valid_machine_id(source_id, 128) => Ok(()),
            Self::SynchronizeClock { request_id, .. } if *request_id > 0 => Ok(()),
            Self::StartCapture { session_id, tracks }
                if valid_session_id(session_id) && valid_tracks(tracks) =>
            {
                Ok(())
            }
            Self::DrainCapture {
                session_id,
                maximum_frames,
            } if valid_session_id(session_id)
                && (1..=MAX_AUDIO_DRAIN_FRAMES).contains(maximum_frames) =>
            {
                Ok(())
            }
            Self::StopCapture { session_id } if valid_session_id(session_id) => Ok(()),
            Self::Hello { .. }
            | Self::RequestPermission { .. }
            | Self::SynchronizeClock { .. }
            | Self::StartCapture { .. }
            | Self::DrainCapture { .. }
            | Self::StopCapture { .. } => Err(AudioMessageError::InvalidHostMessage),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProviderState {
    Starting,
    Ready,
    Capturing,
    Unavailable,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioProviderDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub platform: AudioProviderPlatform,
    pub capabilities: Vec<AudioProviderCapability>,
}

impl AudioProviderDescriptor {
    pub fn validate(&self) -> bool {
        valid_reverse_domain_id(&self.id)
            && valid_text(&self.name, 120)
            && valid_semantic_version(&self.version)
            && !self.capabilities.is_empty()
            && self.capabilities.len() <= MAX_AUDIO_PROVIDER_CAPABILITIES
            && unique_count(&self.capabilities) == self.capabilities.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioStartedTrack {
    pub track_id: String,
    pub source_id: String,
    pub profile: AudioProfileId,
    pub provider_start_monotonic_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCaptureEventKind {
    PermissionRequired,
    PermissionDenied,
    SourceUnavailable,
    SourceChanged,
    Gap,
    Dropout,
    Drift,
    Backpressure,
    EncoderFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioStopReason {
    UserRequested,
    SourceUnavailable,
    PermissionDenied,
    ProviderFailure,
    EncoderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioProviderMessage {
    Hello {
        provider: AudioProviderDescriptor,
    },
    State {
        state: AudioProviderState,
        code: String,
    },
    Sources {
        revision: u64,
        sources: Vec<AudioSourceCapability>,
    },
    ClockSynchronized {
        request_id: u64,
        host_send_monotonic_ns: u64,
        provider_receive_monotonic_ns: u64,
        provider_send_monotonic_ns: u64,
    },
    CaptureStarted {
        session_id: String,
        tracks: Vec<AudioStartedTrack>,
        provider_monotonic_ns: u64,
    },
    PcmFrame {
        session_id: String,
        track_id: String,
        frame_sequence: u64,
        provider_monotonic_ns: u64,
        channels: u8,
        sample_rate_hz: u32,
        frame_count: u16,
        payload_bytes: u32,
    },
    Level {
        session_id: String,
        track_id: String,
        provider_monotonic_ns: u64,
        peak_millidbfs: i32,
        clipped: bool,
    },
    CaptureEvent {
        session_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        track_id: Option<String>,
        provider_monotonic_ns: u64,
        event: AudioCaptureEventKind,
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        affected_frames: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drift_parts_per_million: Option<i32>,
    },
    DrainComplete {
        session_id: String,
        frame_count: u16,
    },
    CaptureStopped {
        session_id: String,
        provider_monotonic_ns: u64,
        reason: AudioStopReason,
    },
}

impl AudioProviderMessage {
    pub fn validate(&self) -> Result<(), AudioMessageError> {
        match self {
            Self::Hello { provider } if provider.validate() => Ok(()),
            Self::State { code, .. } if valid_code(code) => Ok(()),
            Self::Sources { revision, sources }
                if *revision > 0
                    && sources.len() <= MAX_AUDIO_SOURCES
                    && sources.iter().all(|source| source.validate().is_ok())
                    && unique_source_ids(sources) =>
            {
                Ok(())
            }
            Self::ClockSynchronized {
                request_id,
                provider_receive_monotonic_ns,
                provider_send_monotonic_ns,
                ..
            } if *request_id > 0 && provider_receive_monotonic_ns <= provider_send_monotonic_ns => {
                Ok(())
            }
            Self::CaptureStarted {
                session_id, tracks, ..
            } if valid_session_id(session_id) && valid_started_tracks(tracks) => Ok(()),
            Self::PcmFrame {
                session_id,
                track_id,
                frame_sequence,
                channels,
                sample_rate_hz,
                frame_count,
                payload_bytes,
                ..
            } if valid_session_id(session_id)
                && valid_machine_id(track_id, 128)
                && *frame_sequence > 0
                && (1..=8).contains(channels)
                && *sample_rate_hz == AUDIO_WORKING_SAMPLE_RATE_HZ
                && PCM_FRAME_DURATIONS_48KHZ.contains(frame_count)
                && usize::try_from(*payload_bytes).is_ok_and(|length| {
                    length == usize::from(*frame_count) * usize::from(*channels) * 2
                        && length <= MAX_PCM_AUDIO_FRAME_BYTES
                }) =>
            {
                Ok(())
            }
            Self::Level {
                session_id,
                track_id,
                peak_millidbfs,
                ..
            } if valid_session_id(session_id)
                && valid_machine_id(track_id, 128)
                && (MIN_AUDIO_LEVEL_MILLIDBFS..=MAX_AUDIO_LEVEL_MILLIDBFS)
                    .contains(peak_millidbfs) =>
            {
                Ok(())
            }
            Self::DrainComplete {
                session_id,
                frame_count,
            } if valid_session_id(session_id) && *frame_count <= MAX_AUDIO_DRAIN_FRAMES => Ok(()),
            Self::CaptureEvent {
                session_id,
                track_id,
                event,
                code,
                affected_frames,
                drift_parts_per_million,
                ..
            } if valid_session_id(session_id)
                && track_id
                    .as_ref()
                    .is_none_or(|track_id| valid_machine_id(track_id, 128))
                && valid_code(code)
                && valid_event_measurements(*event, *affected_frames, *drift_parts_per_million) =>
            {
                Ok(())
            }
            Self::CaptureStopped { session_id, .. } if valid_session_id(session_id) => Ok(()),
            Self::Hello { .. } => Err(AudioMessageError::InvalidProviderIdentity),
            Self::State { .. } => Err(AudioMessageError::InvalidState),
            Self::Sources { .. } => Err(AudioMessageError::InvalidSources),
            Self::ClockSynchronized { .. } => Err(AudioMessageError::InvalidClock),
            Self::CaptureStarted { .. } => Err(AudioMessageError::InvalidTracks),
            Self::PcmFrame { .. } => Err(AudioMessageError::InvalidPacket),
            Self::Level { .. } => Err(AudioMessageError::InvalidLevel),
            Self::CaptureEvent { .. } => Err(AudioMessageError::InvalidEvent),
            Self::DrainComplete { .. } => Err(AudioMessageError::InvalidPacket),
            Self::CaptureStopped { .. } => Err(AudioMessageError::InvalidSession),
        }
    }

    pub fn declared_body_bytes(&self) -> Option<usize> {
        match self {
            Self::PcmFrame { payload_bytes, .. } => usize::try_from(*payload_bytes).ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioMessageError {
    #[error("audio host message is invalid")]
    InvalidHostMessage,
    #[error("audio provider identity is invalid")]
    InvalidProviderIdentity,
    #[error("audio provider state is invalid")]
    InvalidState,
    #[error("audio source declaration is invalid")]
    InvalidSources,
    #[error("audio clock correlation is invalid")]
    InvalidClock,
    #[error("audio track declaration is invalid")]
    InvalidTracks,
    #[error("audio packet metadata is invalid")]
    InvalidPacket,
    #[error("audio level observation is invalid")]
    InvalidLevel,
    #[error("audio capture event is invalid")]
    InvalidEvent,
    #[error("audio session identity is invalid")]
    InvalidSession,
}

pub fn validate_next_sequence(previous: u64, candidate: u64) -> bool {
    candidate > previous
}

fn valid_tracks(tracks: &[AudioTrackRequest]) -> bool {
    !tracks.is_empty()
        && tracks.len() <= MAX_AUDIO_TRACKS
        && tracks.iter().all(|track| {
            valid_machine_id(&track.track_id, 128) && valid_machine_id(&track.source_id, 128)
        })
        && unique_count_by(tracks, |track| &track.track_id)
        && unique_count_by(tracks, |track| &track.source_id)
}

fn valid_started_tracks(tracks: &[AudioStartedTrack]) -> bool {
    !tracks.is_empty()
        && tracks.len() <= MAX_AUDIO_TRACKS
        && tracks.iter().all(|track| {
            valid_machine_id(&track.track_id, 128) && valid_machine_id(&track.source_id, 128)
        })
        && unique_count_by(tracks, |track| &track.track_id)
        && unique_count_by(tracks, |track| &track.source_id)
}

fn unique_source_ids(sources: &[AudioSourceCapability]) -> bool {
    unique_count_by(sources, |source| &source.id)
}

fn valid_event_measurements(
    event: AudioCaptureEventKind,
    affected_frames: Option<u64>,
    drift_parts_per_million: Option<i32>,
) -> bool {
    match event {
        AudioCaptureEventKind::Gap
        | AudioCaptureEventKind::Dropout
        | AudioCaptureEventKind::Backpressure => {
            affected_frames.is_some_and(|frames| (1..=MAX_AUDIO_EVENT_FRAMES).contains(&frames))
                && drift_parts_per_million.is_none()
        }
        AudioCaptureEventKind::Drift => {
            affected_frames.is_none()
                && drift_parts_per_million.is_some_and(|drift| {
                    (-MAX_ABSOLUTE_AUDIO_DRIFT_PPM..=MAX_ABSOLUTE_AUDIO_DRIFT_PPM).contains(&drift)
                })
        }
        _ => affected_frames.is_none() && drift_parts_per_million.is_none(),
    }
}

fn valid_session_id(value: &str) -> bool {
    valid_machine_id(value, 128)
}

fn valid_code(value: &str) -> bool {
    valid_machine_id(value, 96)
}

fn valid_machine_id(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, b'.' | b'_' | b':' | b'-')
        })
}

fn valid_reverse_domain_id(value: &str) -> bool {
    let segments = value.split('.').collect::<Vec<_>>();
    segments.len() >= 3
        && value.len() <= 255
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= 63
                && segment.bytes().all(|character| {
                    character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || character == b'-'
                })
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && segment
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

fn valid_semantic_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|character| character.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn unique_count<T: Eq + std::hash::Hash>(values: &[T]) -> usize {
    values.iter().collect::<HashSet<_>>().len()
}

fn unique_count_by<T, K: Eq + std::hash::Hash + ?Sized>(
    values: &[T],
    key: impl Fn(&T) -> &K,
) -> bool {
    let mut keys = HashSet::new();
    values.iter().all(|value| keys.insert(key(value)))
}

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};
use thiserror::Error;
use wyrmgrid_domain::{AUDIO_WORKING_SAMPLE_RATE_HZ, AudioProfileId};

pub const AUDIO_CODEC_PROTOCOL_VERSION: u32 = 1;
pub const AUDIO_CODEC_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_CODEC_CONTROL_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_CODEC_PCM_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_CODEC_PACKET_BYTES: usize = 16 * 1024;
pub const MAX_CODEC_PROFILES: usize = 16;
pub const MAX_CODEC_TRACKS: usize = 8;
pub const PCM_SAMPLE_BYTES: usize = 2;
pub const CODEC_PACKET_DURATIONS_48KHZ: [u16; 6] = [120, 240, 480, 960, 1_920, 2_880];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodecPlatform {
    WindowsX86_64,
    LinuxX86_64,
    MacosAarch64,
    MacosX86_64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodecCapability {
    EncodePcmS16le,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCodecProfile {
    pub id: AudioProfileId,
    pub codec_id: String,
    pub media_type: String,
    pub channels: u8,
    pub sample_rate_hz: u32,
    pub target_bitrate_bps: u32,
    pub packet_duration_48khz_frames: u16,
}

impl AudioCodecProfile {
    pub fn validate(&self) -> bool {
        let expected = self.id.spec();
        self.channels == expected.channels
            && self.sample_rate_hz == expected.sample_rate_hz
            && self.sample_rate_hz == AUDIO_WORKING_SAMPLE_RATE_HZ
            && valid_machine_id(&self.codec_id, 96)
            && valid_media_type(&self.media_type)
            && (8_000..=10_000_000).contains(&self.target_bitrate_bps)
            && CODEC_PACKET_DURATIONS_48KHZ.contains(&self.packet_duration_48khz_frames)
    }

    pub fn estimated_encoded_bytes(&self, duration_seconds: u64) -> Option<u64> {
        u64::from(self.target_bitrate_bps)
            .checked_mul(duration_seconds)
            .map(|bits| bits / 8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCodecManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub codec_protocol_version: u32,
    pub author: String,
    pub entry_point: String,
    pub platforms: Vec<AudioCodecPlatform>,
    pub capabilities: Vec<AudioCodecCapability>,
    pub profiles: Vec<AudioCodecProfile>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecManifestError {
    #[error("unsupported audio codec manifest schema")]
    UnsupportedSchema,
    #[error("audio codec id must use reverse-domain notation")]
    InvalidId,
    #[error("audio codec metadata is invalid")]
    InvalidMetadata,
    #[error("audio codec uses an unsupported protocol")]
    UnsupportedProtocol,
    #[error("audio codec entry point must be a safe relative path")]
    UnsafeEntryPoint,
    #[error("audio codec declarations are invalid or duplicated")]
    InvalidDeclaration,
}

impl AudioCodecManifest {
    pub fn validate(&self) -> Result<(), AudioCodecManifestError> {
        if self.schema_version != AUDIO_CODEC_MANIFEST_SCHEMA_VERSION {
            return Err(AudioCodecManifestError::UnsupportedSchema);
        }
        if !valid_reverse_domain_id(&self.id) {
            return Err(AudioCodecManifestError::InvalidId);
        }
        if self
            .schema
            .as_ref()
            .is_some_and(|schema| !valid_text(schema, 256))
            || !valid_text(&self.name, 120)
            || !valid_text(&self.author, 120)
            || !valid_semantic_version(&self.version)
        {
            return Err(AudioCodecManifestError::InvalidMetadata);
        }
        if self.codec_protocol_version != AUDIO_CODEC_PROTOCOL_VERSION {
            return Err(AudioCodecManifestError::UnsupportedProtocol);
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
            return Err(AudioCodecManifestError::UnsafeEntryPoint);
        }
        let profile_ids = self
            .profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<HashSet<_>>();
        if self.platforms.is_empty()
            || self.capabilities != [AudioCodecCapability::EncodePcmS16le]
            || self.profiles.is_empty()
            || self.profiles.len() > MAX_CODEC_PROFILES
            || profile_ids.len() != self.profiles.len()
            || unique_count(&self.platforms) != self.platforms.len()
            || self.profiles.iter().any(|profile| !profile.validate())
        {
            return Err(AudioCodecManifestError::InvalidDeclaration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodecEnvelope<T> {
    pub protocol_version: u32,
    pub sequence: u64,
    pub payload: T,
}

impl<T> CodecEnvelope<T> {
    pub fn new(sequence: u64, payload: T) -> Self {
        Self {
            protocol_version: AUDIO_CODEC_PROTOCOL_VERSION,
            sequence,
            payload,
        }
    }

    pub fn validate_header(&self) -> Result<(), CodecEnvelopeError> {
        if self.protocol_version != AUDIO_CODEC_PROTOCOL_VERSION {
            return Err(CodecEnvelopeError::UnsupportedProtocolVersion);
        }
        if self.sequence == 0 {
            return Err(CodecEnvelopeError::InvalidSequence);
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum CodecEnvelopeError {
    #[error("unsupported audio codec protocol version")]
    UnsupportedProtocolVersion,
    #[error("audio codec message sequence must be greater than zero")]
    InvalidSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PcmSampleFormat {
    S16le,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioCodecHostMessage {
    Hello {
        host_version: String,
        codec_provider_id: String,
    },
    StartTrack {
        session_id: String,
        track_id: String,
        profile: AudioProfileId,
    },
    EncodePcm {
        session_id: String,
        track_id: String,
        frame_sequence: u64,
        provider_monotonic_ns: u64,
        sample_format: PcmSampleFormat,
        channels: u8,
        sample_rate_hz: u32,
        frame_count: u16,
        payload_bytes: u32,
    },
    StopTrack {
        session_id: String,
        track_id: String,
    },
    Shutdown,
}

impl AudioCodecHostMessage {
    pub fn validate(&self) -> Result<(), AudioCodecMessageError> {
        match self {
            Self::Hello {
                host_version,
                codec_provider_id,
            } if valid_semantic_version(host_version)
                && valid_reverse_domain_id(codec_provider_id) =>
            {
                Ok(())
            }
            Self::StartTrack {
                session_id,
                track_id,
                ..
            }
            | Self::StopTrack {
                session_id,
                track_id,
            } if valid_machine_id(session_id, 128) && valid_machine_id(track_id, 128) => Ok(()),
            Self::EncodePcm {
                session_id,
                track_id,
                frame_sequence,
                channels,
                sample_rate_hz,
                frame_count,
                payload_bytes,
                ..
            } if valid_machine_id(session_id, 128)
                && valid_machine_id(track_id, 128)
                && *frame_sequence > 0
                && (1..=8).contains(channels)
                && *sample_rate_hz == AUDIO_WORKING_SAMPLE_RATE_HZ
                && CODEC_PACKET_DURATIONS_48KHZ.contains(frame_count)
                && usize::try_from(*payload_bytes).is_ok_and(|payload_bytes| {
                    payload_bytes
                        == usize::from(*frame_count) * usize::from(*channels) * PCM_SAMPLE_BYTES
                        && payload_bytes <= MAX_CODEC_PCM_FRAME_BYTES
                }) =>
            {
                Ok(())
            }
            Self::Shutdown => Ok(()),
            _ => Err(AudioCodecMessageError::InvalidHostMessage),
        }
    }

    pub fn declared_body_bytes(&self) -> Option<usize> {
        match self {
            Self::EncodePcm { payload_bytes, .. } => usize::try_from(*payload_bytes).ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodecState {
    Starting,
    Ready,
    Encoding,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCodecDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub platform: AudioCodecPlatform,
    pub profiles: Vec<AudioCodecProfile>,
}

impl AudioCodecDescriptor {
    pub fn validate(&self) -> bool {
        valid_reverse_domain_id(&self.id)
            && valid_text(&self.name, 120)
            && valid_semantic_version(&self.version)
            && !self.profiles.is_empty()
            && self.profiles.len() <= MAX_CODEC_PROFILES
            && self.profiles.iter().all(AudioCodecProfile::validate)
            && unique_count_by(&self.profiles, |profile| profile.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioCodecProviderMessage {
    Hello {
        codec: AudioCodecDescriptor,
    },
    State {
        state: AudioCodecState,
        code: String,
    },
    TrackStarted {
        session_id: String,
        track_id: String,
        profile: AudioProfileId,
    },
    EncodedPacket {
        session_id: String,
        track_id: String,
        packet_sequence: u64,
        provider_monotonic_ns: u64,
        duration_48khz_frames: u16,
        payload_bytes: u32,
    },
    TrackStopped {
        session_id: String,
        track_id: String,
    },
    Error {
        code: String,
    },
}

impl AudioCodecProviderMessage {
    pub fn validate(&self) -> Result<(), AudioCodecMessageError> {
        match self {
            Self::Hello { codec } if codec.validate() => Ok(()),
            Self::State { code, .. } | Self::Error { code } if valid_machine_id(code, 96) => Ok(()),
            Self::TrackStarted {
                session_id,
                track_id,
                ..
            }
            | Self::TrackStopped {
                session_id,
                track_id,
            } if valid_machine_id(session_id, 128) && valid_machine_id(track_id, 128) => Ok(()),
            Self::EncodedPacket {
                session_id,
                track_id,
                packet_sequence,
                duration_48khz_frames,
                payload_bytes,
                ..
            } if valid_machine_id(session_id, 128)
                && valid_machine_id(track_id, 128)
                && *packet_sequence > 0
                && CODEC_PACKET_DURATIONS_48KHZ.contains(duration_48khz_frames)
                && usize::try_from(*payload_bytes)
                    .is_ok_and(|length| (1..=MAX_CODEC_PACKET_BYTES).contains(&length)) =>
            {
                Ok(())
            }
            _ => Err(AudioCodecMessageError::InvalidProviderMessage),
        }
    }

    pub fn declared_body_bytes(&self) -> Option<usize> {
        match self {
            Self::EncodedPacket { payload_bytes, .. } => usize::try_from(*payload_bytes).ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecMessageError {
    #[error("audio codec host message is invalid")]
    InvalidHostMessage,
    #[error("audio codec provider message is invalid")]
    InvalidProviderMessage,
}

pub fn validate_next_codec_sequence(previous: u64, candidate: u64) -> bool {
    candidate > previous
}

fn valid_reverse_domain_id(value: &str) -> bool {
    let segments = value.split('.').collect::<Vec<_>>();
    segments.len() >= 3
        && value.len() <= 255
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= 63
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                && segment
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn valid_semantic_version(value: &str) -> bool {
    let components = value.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && component.bytes().all(|byte| byte.is_ascii_digit())
                && (component == &"0" || !component.starts_with('0'))
        })
}

fn valid_machine_id(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_media_type(value: &str) -> bool {
    value.len() <= 120
        && value
            .split_once('/')
            .is_some_and(|(kind, subtype)| valid_token(kind) && valid_token(subtype))
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                )
        })
}

fn unique_count<T: Eq + std::hash::Hash>(items: &[T]) -> usize {
    items.iter().collect::<HashSet<_>>().len()
}

fn unique_count_by<T, K: Eq + std::hash::Hash>(items: &[T], key: impl Fn(&T) -> K) -> bool {
    let mut keys = HashSet::with_capacity(items.len());
    items.iter().all(|item| keys.insert(key(item)))
}

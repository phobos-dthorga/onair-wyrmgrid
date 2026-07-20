use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

use crate::valid_text;

pub const AUDIO_SOURCE_SCHEMA_VERSION: u32 = 1;
pub const OPUS_SAMPLE_RATE_HZ: u32 = 48_000;
pub const MIN_AUDIO_NATIVE_SAMPLE_RATE_HZ: u32 = 8_000;
pub const MAX_AUDIO_NATIVE_SAMPLE_RATE_HZ: u32 = 384_000;
pub const MAX_AUDIO_SOURCE_CHANNELS: u8 = 8;
pub const MAX_AUDIO_SOURCE_PROFILES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSourceRole {
    MicrophoneInput,
    ApplicationOutput,
    OutputEndpoint,
    SimulatorMasterMix,
    IsolatedCom1,
    IsolatedCom2,
    PilotRadio,
    CopilotRadio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSourceDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSourceTruth {
    Isolated,
    MixedOutput,
    MetadataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSourceAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPermissionState {
    NotRequired,
    PromptRequired,
    Granted,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioSourceOrigin {
    OperatingSystem,
    Simulator { identifier: String },
    ExternalApplication { identifier: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioOpusProfileId {
    PilotMicrophoneV1,
    IsolatedVoiceV1,
    MixedStereoV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioOpusProfile {
    pub id: AudioOpusProfileId,
    pub channels: u8,
    pub sample_rate_hz: u32,
    pub target_bitrate_bps: u32,
}

impl AudioOpusProfileId {
    pub const fn spec(self) -> AudioOpusProfile {
        match self {
            Self::PilotMicrophoneV1 => AudioOpusProfile {
                id: self,
                channels: 1,
                sample_rate_hz: OPUS_SAMPLE_RATE_HZ,
                target_bitrate_bps: 48_000,
            },
            Self::IsolatedVoiceV1 => AudioOpusProfile {
                id: self,
                channels: 1,
                sample_rate_hz: OPUS_SAMPLE_RATE_HZ,
                target_bitrate_bps: 32_000,
            },
            Self::MixedStereoV1 => AudioOpusProfile {
                id: self,
                channels: 2,
                sample_rate_hz: OPUS_SAMPLE_RATE_HZ,
                target_bitrate_bps: 128_000,
            },
        }
    }
}

impl AudioOpusProfile {
    pub fn estimated_encoded_bytes(self, duration_seconds: u64) -> Option<u64> {
        u64::from(self.target_bitrate_bps)
            .checked_mul(duration_seconds)
            .map(|bits| bits / 8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSourceCapability {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub role: AudioSourceRole,
    pub direction: AudioSourceDirection,
    pub truth: AudioSourceTruth,
    pub availability: AudioSourceAvailability,
    pub permission: AudioPermissionState,
    pub channels: u8,
    pub native_sample_rate_hz: u32,
    pub supported_profiles: Vec<AudioOpusProfileId>,
    pub supports_hot_plug: bool,
    pub origin: AudioSourceOrigin,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioSourceCapabilityError {
    #[error("unsupported audio source schema version")]
    UnsupportedSchemaVersion,
    #[error("audio source identifier or display label is invalid")]
    InvalidIdentity,
    #[error("audio source origin is invalid")]
    InvalidOrigin,
    #[error("audio source channel or sample-rate declaration is invalid")]
    InvalidFormat,
    #[error("audio source profile declaration is invalid")]
    InvalidProfiles,
}

impl AudioSourceCapability {
    pub fn validate(&self) -> Result<(), AudioSourceCapabilityError> {
        if self.schema_version != AUDIO_SOURCE_SCHEMA_VERSION {
            return Err(AudioSourceCapabilityError::UnsupportedSchemaVersion);
        }
        if !valid_audio_identifier(&self.id) || !valid_text(&self.display_name, 160) {
            return Err(AudioSourceCapabilityError::InvalidIdentity);
        }
        if !self.origin.is_valid() {
            return Err(AudioSourceCapabilityError::InvalidOrigin);
        }
        if !(1..=MAX_AUDIO_SOURCE_CHANNELS).contains(&self.channels)
            || !(MIN_AUDIO_NATIVE_SAMPLE_RATE_HZ..=MAX_AUDIO_NATIVE_SAMPLE_RATE_HZ)
                .contains(&self.native_sample_rate_hz)
        {
            return Err(AudioSourceCapabilityError::InvalidFormat);
        }

        let unique_profiles = self.supported_profiles.iter().collect::<HashSet<_>>().len();
        let profiles_are_valid = if self.truth == AudioSourceTruth::MetadataOnly {
            self.supported_profiles.is_empty()
        } else {
            !self.supported_profiles.is_empty()
                && self.supported_profiles.len() <= MAX_AUDIO_SOURCE_PROFILES
                && unique_profiles == self.supported_profiles.len()
                && self
                    .supported_profiles
                    .iter()
                    .all(|profile| profile.spec().channels <= self.channels)
        };
        if !profiles_are_valid {
            return Err(AudioSourceCapabilityError::InvalidProfiles);
        }
        Ok(())
    }

    pub fn is_capture_ready(&self) -> bool {
        self.truth != AudioSourceTruth::MetadataOnly
            && self.availability == AudioSourceAvailability::Available
            && matches!(
                self.permission,
                AudioPermissionState::NotRequired | AudioPermissionState::Granted
            )
            && !self.supported_profiles.is_empty()
    }
}

impl AudioSourceOrigin {
    fn is_valid(&self) -> bool {
        match self {
            Self::OperatingSystem => true,
            Self::Simulator { identifier } | Self::ExternalApplication { identifier } => {
                valid_audio_identifier(identifier)
            }
        }
    }
}

fn valid_audio_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, b'.' | b'_' | b':' | b'-')
        })
}

#[cfg(test)]
#[path = "tests/audio.rs"]
mod tests;

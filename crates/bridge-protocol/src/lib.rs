//! Language-neutral contracts between WyrmGrid and simulator provider sidecars.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Component, Path};
use thiserror::Error;
use wyrmgrid_domain::SimulatorTelemetrySnapshot;

pub const BRIDGE_PROTOCOL_VERSION: u32 = 1;
pub const PROVIDER_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_BRIDGE_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_TELEMETRY_FREQUENCY_HZ: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeCapability {
    TelemetryRead,
    ActivePlanRead,
    FlightPlanLoad,
    CommandExecute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPlatform {
    WindowsX86_64,
    LinuxX86_64,
    MacosAarch64,
    MacosX86_64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub bridge_protocol_version: u32,
    pub author: String,
    pub entry_point: String,
    pub platforms: Vec<ProviderPlatform>,
    pub simulators: Vec<String>,
    pub capabilities: Vec<BridgeCapability>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ProviderManifestError {
    #[error("unsupported simulator provider manifest schema")]
    UnsupportedSchema,
    #[error("simulator provider id must use reverse-domain notation")]
    InvalidId,
    #[error("simulator provider metadata is invalid")]
    InvalidMetadata,
    #[error("simulator provider uses an unsupported Bridge protocol")]
    UnsupportedProtocol,
    #[error("simulator provider entry point must be a safe relative path")]
    UnsafeEntryPoint,
    #[error("simulator provider must declare unique platforms, simulators, and capabilities")]
    InvalidDeclaration,
}

impl ProviderManifest {
    pub fn validate(&self) -> Result<(), ProviderManifestError> {
        if self.schema_version != PROVIDER_MANIFEST_SCHEMA_VERSION {
            return Err(ProviderManifestError::UnsupportedSchema);
        }
        if self
            .schema
            .as_ref()
            .is_some_and(|schema| !valid_text(schema, 256))
        {
            return Err(ProviderManifestError::InvalidMetadata);
        }
        if !valid_reverse_domain_id(&self.id) {
            return Err(ProviderManifestError::InvalidId);
        }
        if !valid_text(&self.name, 120)
            || !valid_text(&self.author, 120)
            || !valid_semantic_version(&self.version)
        {
            return Err(ProviderManifestError::InvalidMetadata);
        }
        if self.bridge_protocol_version != BRIDGE_PROTOCOL_VERSION {
            return Err(ProviderManifestError::UnsupportedProtocol);
        }
        let entry_point = Path::new(&self.entry_point);
        if self.entry_point.trim().is_empty()
            || entry_point.is_absolute()
            || entry_point.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ProviderManifestError::UnsafeEntryPoint);
        }
        if self.platforms.is_empty()
            || self.simulators.is_empty()
            || self.capabilities.is_empty()
            || unique_count(&self.platforms) != self.platforms.len()
            || unique_count(&self.capabilities) != self.capabilities.len()
            || self.simulators.iter().any(|value| !valid_identifier(value))
            || self.simulators.iter().collect::<HashSet<_>>().len() != self.simulators.len()
        {
            return Err(ProviderManifestError::InvalidDeclaration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeEnvelope<T> {
    pub protocol_version: u32,
    pub sequence: u64,
    pub payload: T,
}

impl<T> BridgeEnvelope<T> {
    pub fn new(sequence: u64, payload: T) -> Self {
        Self {
            protocol_version: BRIDGE_PROTOCOL_VERSION,
            sequence,
            payload,
        }
    }

    pub fn validate_header(&self) -> Result<(), BridgeEnvelopeError> {
        if self.protocol_version != BRIDGE_PROTOCOL_VERSION {
            return Err(BridgeEnvelopeError::UnsupportedProtocolVersion);
        }
        if self.sequence == 0 {
            return Err(BridgeEnvelopeError::InvalidSequence);
        }
        Ok(())
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum BridgeEnvelopeError {
    #[error("unsupported Bridge protocol version")]
    UnsupportedProtocolVersion,
    #[error("Bridge message sequence must be greater than zero")]
    InvalidSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeHostMessage {
    Hello {
        host_version: String,
        provider_id: String,
        requested_capabilities: Vec<BridgeCapability>,
    },
    StartTelemetry {
        maximum_frequency_hz: u8,
    },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderConnectionState {
    Starting,
    WaitingForSimulator,
    Connected,
    Disconnected,
    Stopped,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub simulator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulator_version: Option<String>,
    pub architecture: String,
    pub capabilities: Vec<BridgeCapability>,
}

impl ProviderDescriptor {
    pub fn validate(&self) -> bool {
        valid_reverse_domain_id(&self.id)
            && valid_text(&self.name, 120)
            && valid_semantic_version(&self.version)
            && valid_identifier(&self.simulator)
            && self
                .simulator_version
                .as_ref()
                .is_none_or(|version| valid_text(version, 64))
            && valid_identifier(&self.architecture)
            && !self.capabilities.is_empty()
            && unique_count(&self.capabilities) == self.capabilities.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeProviderMessage {
    Hello {
        provider: ProviderDescriptor,
    },
    State {
        state: ProviderConnectionState,
        code: String,
    },
    Telemetry {
        snapshot: SimulatorTelemetrySnapshot,
    },
}

#[derive(Debug, Error)]
pub enum BridgeFrameError {
    #[error("Bridge stream closed")]
    Closed,
    #[error("Bridge frame header is incomplete")]
    TruncatedHeader,
    #[error("Bridge frame exceeds the {maximum}-byte limit")]
    TooLarge { maximum: usize },
    #[error("Bridge frame is empty")]
    Empty,
    #[error("Bridge stream I/O failed")]
    Io(#[source] std::io::Error),
    #[error("Bridge message could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("Bridge message could not be decoded")]
    Decode(#[source] serde_json::Error),
}

pub fn write_frame<W: Write, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<(), BridgeFrameError> {
    let payload = serde_json::to_vec(message).map_err(BridgeFrameError::Encode)?;
    if payload.is_empty() {
        return Err(BridgeFrameError::Empty);
    }
    if payload.len() > MAX_BRIDGE_FRAME_BYTES {
        return Err(BridgeFrameError::TooLarge {
            maximum: MAX_BRIDGE_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| BridgeFrameError::TooLarge {
        maximum: MAX_BRIDGE_FRAME_BYTES,
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .map_err(BridgeFrameError::Io)?;
    writer.write_all(&payload).map_err(BridgeFrameError::Io)?;
    writer.flush().map_err(BridgeFrameError::Io)
}

pub fn read_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T, BridgeFrameError> {
    let mut header = [0_u8; 4];
    match reader.read(&mut header[..1]) {
        Ok(0) => return Err(BridgeFrameError::Closed),
        Ok(1) => {}
        Ok(_) => unreachable!("one-byte read returned more than one byte"),
        Err(error) => return Err(BridgeFrameError::Io(error)),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::UnexpectedEof => BridgeFrameError::TruncatedHeader,
            _ => BridgeFrameError::Io(error),
        })?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 {
        return Err(BridgeFrameError::Empty);
    }
    if length > MAX_BRIDGE_FRAME_BYTES {
        return Err(BridgeFrameError::TooLarge {
            maximum: MAX_BRIDGE_FRAME_BYTES,
        });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(BridgeFrameError::Io)?;
    serde_json::from_slice(&payload).map_err(BridgeFrameError::Decode)
}

pub fn valid_state_code(value: &str) -> bool {
    valid_identifier(value) && value.len() <= 96
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

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.bytes().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, b'_' | b'-' | b'.')
        })
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn unique_count<T: Eq + std::hash::Hash>(values: &[T]) -> usize {
    values.iter().collect::<HashSet<_>>().len()
}

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;

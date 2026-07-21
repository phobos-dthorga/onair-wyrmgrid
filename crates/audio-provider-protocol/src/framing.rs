use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use thiserror::Error;

use crate::{
    AudioEnvelope, AudioHostMessage, AudioProviderMessage, MAX_AUDIO_CONTROL_FRAME_BYTES,
    MAX_PCM_AUDIO_FRAME_BYTES,
};

#[derive(Debug, Error)]
pub enum AudioFrameError {
    #[error("audio provider stream closed")]
    Closed,
    #[error("audio provider frame header is incomplete")]
    TruncatedHeader,
    #[error("audio provider control header exceeds the {maximum}-byte limit")]
    HeaderTooLarge { maximum: usize },
    #[error("audio provider binary body exceeds the {maximum}-byte limit")]
    BodyTooLarge { maximum: usize },
    #[error("audio provider control header is empty")]
    EmptyHeader,
    #[error("audio provider stream I/O failed")]
    Io(#[source] std::io::Error),
    #[error("audio provider control header could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("audio provider control header could not be decoded")]
    Decode(#[source] serde_json::Error),
    #[error("audio provider envelope header is invalid")]
    InvalidEnvelope,
    #[error("audio provider message is invalid")]
    InvalidMessage,
    #[error("only a PCM frame may carry a binary body")]
    UnexpectedBody,
    #[error("PCM frame binary body is missing")]
    MissingBody,
    #[error("PCM frame binary body length does not match its declaration")]
    BodyLengthMismatch,
}

pub fn write_host_frame<W: Write>(
    writer: &mut W,
    envelope: &AudioEnvelope<AudioHostMessage>,
) -> Result<(), AudioFrameError> {
    envelope
        .validate_header()
        .map_err(|_| AudioFrameError::InvalidEnvelope)?;
    envelope
        .payload
        .validate()
        .map_err(|_| AudioFrameError::InvalidMessage)?;
    write_json_frame(writer, envelope)
}

pub fn read_host_frame<R: Read>(
    reader: &mut R,
) -> Result<AudioEnvelope<AudioHostMessage>, AudioFrameError> {
    let envelope: AudioEnvelope<AudioHostMessage> = read_json_frame(reader)?;
    envelope
        .validate_header()
        .map_err(|_| AudioFrameError::InvalidEnvelope)?;
    envelope
        .payload
        .validate()
        .map_err(|_| AudioFrameError::InvalidMessage)?;
    Ok(envelope)
}

pub fn write_provider_frame<W: Write>(
    writer: &mut W,
    envelope: &AudioEnvelope<AudioProviderMessage>,
    body: &[u8],
) -> Result<(), AudioFrameError> {
    envelope
        .validate_header()
        .map_err(|_| AudioFrameError::InvalidEnvelope)?;
    envelope
        .payload
        .validate()
        .map_err(|_| AudioFrameError::InvalidMessage)?;
    validate_body(&envelope.payload, body.len())?;

    let header = serde_json::to_vec(envelope).map_err(AudioFrameError::Encode)?;
    validate_lengths(header.len(), body.len())?;
    let header_length =
        u32::try_from(header.len()).map_err(|_| AudioFrameError::HeaderTooLarge {
            maximum: MAX_AUDIO_CONTROL_FRAME_BYTES,
        })?;
    let body_length = u32::try_from(body.len()).map_err(|_| AudioFrameError::BodyTooLarge {
        maximum: MAX_PCM_AUDIO_FRAME_BYTES,
    })?;
    writer
        .write_all(&header_length.to_be_bytes())
        .map_err(AudioFrameError::Io)?;
    writer
        .write_all(&body_length.to_be_bytes())
        .map_err(AudioFrameError::Io)?;
    writer.write_all(&header).map_err(AudioFrameError::Io)?;
    writer.write_all(body).map_err(AudioFrameError::Io)?;
    writer.flush().map_err(AudioFrameError::Io)
}

pub fn read_provider_frame<R: Read>(
    reader: &mut R,
) -> Result<(AudioEnvelope<AudioProviderMessage>, Vec<u8>), AudioFrameError> {
    let (header_length, body_length) = read_provider_lengths(reader)?;
    validate_lengths(header_length, body_length)?;

    let mut header = vec![0_u8; header_length];
    reader
        .read_exact(&mut header)
        .map_err(AudioFrameError::Io)?;
    let envelope: AudioEnvelope<AudioProviderMessage> =
        serde_json::from_slice(&header).map_err(AudioFrameError::Decode)?;
    envelope
        .validate_header()
        .map_err(|_| AudioFrameError::InvalidEnvelope)?;
    envelope
        .payload
        .validate()
        .map_err(|_| AudioFrameError::InvalidMessage)?;
    validate_body(&envelope.payload, body_length)?;

    let mut body = vec![0_u8; body_length];
    reader.read_exact(&mut body).map_err(AudioFrameError::Io)?;
    Ok((envelope, body))
}

fn write_json_frame<W: Write, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<(), AudioFrameError> {
    let payload = serde_json::to_vec(message).map_err(AudioFrameError::Encode)?;
    if payload.is_empty() {
        return Err(AudioFrameError::EmptyHeader);
    }
    if payload.len() > MAX_AUDIO_CONTROL_FRAME_BYTES {
        return Err(AudioFrameError::HeaderTooLarge {
            maximum: MAX_AUDIO_CONTROL_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| AudioFrameError::HeaderTooLarge {
        maximum: MAX_AUDIO_CONTROL_FRAME_BYTES,
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .map_err(AudioFrameError::Io)?;
    writer.write_all(&payload).map_err(AudioFrameError::Io)?;
    writer.flush().map_err(AudioFrameError::Io)
}

fn read_json_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T, AudioFrameError> {
    let length = read_single_length(reader)?;
    if length == 0 {
        return Err(AudioFrameError::EmptyHeader);
    }
    if length > MAX_AUDIO_CONTROL_FRAME_BYTES {
        return Err(AudioFrameError::HeaderTooLarge {
            maximum: MAX_AUDIO_CONTROL_FRAME_BYTES,
        });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(AudioFrameError::Io)?;
    serde_json::from_slice(&payload).map_err(AudioFrameError::Decode)
}

fn read_single_length<R: Read>(reader: &mut R) -> Result<usize, AudioFrameError> {
    let mut header = [0_u8; 4];
    read_header(reader, &mut header)?;
    Ok(u32::from_be_bytes(header) as usize)
}

fn read_provider_lengths<R: Read>(reader: &mut R) -> Result<(usize, usize), AudioFrameError> {
    let mut header = [0_u8; 8];
    read_header(reader, &mut header)?;
    let header_length = u32::from_be_bytes(header[..4].try_into().expect("fixed header"));
    let body_length = u32::from_be_bytes(header[4..].try_into().expect("fixed header"));
    Ok((header_length as usize, body_length as usize))
}

fn read_header<R: Read>(reader: &mut R, header: &mut [u8]) -> Result<(), AudioFrameError> {
    match reader.read(&mut header[..1]) {
        Ok(0) => return Err(AudioFrameError::Closed),
        Ok(1) => {}
        Ok(_) => unreachable!("one-byte read returned more than one byte"),
        Err(error) => return Err(AudioFrameError::Io(error)),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::UnexpectedEof => AudioFrameError::TruncatedHeader,
            _ => AudioFrameError::Io(error),
        })
}

fn validate_lengths(header_length: usize, body_length: usize) -> Result<(), AudioFrameError> {
    if header_length == 0 {
        return Err(AudioFrameError::EmptyHeader);
    }
    if header_length > MAX_AUDIO_CONTROL_FRAME_BYTES {
        return Err(AudioFrameError::HeaderTooLarge {
            maximum: MAX_AUDIO_CONTROL_FRAME_BYTES,
        });
    }
    if body_length > MAX_PCM_AUDIO_FRAME_BYTES {
        return Err(AudioFrameError::BodyTooLarge {
            maximum: MAX_PCM_AUDIO_FRAME_BYTES,
        });
    }
    Ok(())
}

fn validate_body(
    message: &AudioProviderMessage,
    body_length: usize,
) -> Result<(), AudioFrameError> {
    match message.declared_body_bytes() {
        Some(0) => Err(AudioFrameError::MissingBody),
        Some(_) if body_length == 0 => Err(AudioFrameError::MissingBody),
        Some(expected) if expected != body_length => Err(AudioFrameError::BodyLengthMismatch),
        Some(_) => Ok(()),
        None if body_length > 0 => Err(AudioFrameError::UnexpectedBody),
        None => Ok(()),
    }
}

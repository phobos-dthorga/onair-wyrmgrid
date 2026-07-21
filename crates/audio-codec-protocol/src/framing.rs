use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use thiserror::Error;

use crate::{
    AudioCodecHostMessage, AudioCodecProviderMessage, CodecEnvelope, MAX_CODEC_CONTROL_FRAME_BYTES,
    MAX_CODEC_PACKET_BYTES, MAX_CODEC_PCM_FRAME_BYTES,
};

#[derive(Debug, Error)]
pub enum CodecFrameError {
    #[error("audio codec stream closed")]
    Closed,
    #[error("audio codec frame header is incomplete")]
    TruncatedHeader,
    #[error("audio codec frame exceeds its limit")]
    TooLarge,
    #[error("audio codec frame is empty")]
    Empty,
    #[error("audio codec stream I/O failed")]
    Io(#[source] std::io::Error),
    #[error("audio codec frame could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("audio codec frame could not be decoded")]
    Decode(#[source] serde_json::Error),
    #[error("audio codec envelope or body is invalid")]
    Invalid,
}

pub fn write_codec_host_frame<W: Write>(
    writer: &mut W,
    envelope: &CodecEnvelope<AudioCodecHostMessage>,
    body: &[u8],
) -> Result<(), CodecFrameError> {
    envelope
        .validate_header()
        .map_err(|_| CodecFrameError::Invalid)?;
    envelope
        .payload
        .validate()
        .map_err(|_| CodecFrameError::Invalid)?;
    let expected = envelope.payload.declared_body_bytes().unwrap_or(0);
    if expected != body.len() || body.len() > MAX_CODEC_PCM_FRAME_BYTES {
        return Err(CodecFrameError::Invalid);
    }
    write_frame(writer, envelope, body)
}

pub fn read_codec_host_frame<R: Read>(
    reader: &mut R,
) -> Result<(CodecEnvelope<AudioCodecHostMessage>, Vec<u8>), CodecFrameError> {
    let (envelope, body): (CodecEnvelope<AudioCodecHostMessage>, Vec<u8>) =
        read_frame(reader, MAX_CODEC_PCM_FRAME_BYTES)?;
    envelope
        .validate_header()
        .map_err(|_| CodecFrameError::Invalid)?;
    envelope
        .payload
        .validate()
        .map_err(|_| CodecFrameError::Invalid)?;
    if envelope.payload.declared_body_bytes().unwrap_or(0) != body.len() {
        return Err(CodecFrameError::Invalid);
    }
    Ok((envelope, body))
}

pub fn write_codec_provider_frame<W: Write>(
    writer: &mut W,
    envelope: &CodecEnvelope<AudioCodecProviderMessage>,
    body: &[u8],
) -> Result<(), CodecFrameError> {
    envelope
        .validate_header()
        .map_err(|_| CodecFrameError::Invalid)?;
    envelope
        .payload
        .validate()
        .map_err(|_| CodecFrameError::Invalid)?;
    let expected = envelope.payload.declared_body_bytes().unwrap_or(0);
    if expected != body.len() || body.len() > MAX_CODEC_PACKET_BYTES {
        return Err(CodecFrameError::Invalid);
    }
    write_frame(writer, envelope, body)
}

pub fn read_codec_provider_frame<R: Read>(
    reader: &mut R,
) -> Result<(CodecEnvelope<AudioCodecProviderMessage>, Vec<u8>), CodecFrameError> {
    let (envelope, body): (CodecEnvelope<AudioCodecProviderMessage>, Vec<u8>) =
        read_frame(reader, MAX_CODEC_PACKET_BYTES)?;
    envelope
        .validate_header()
        .map_err(|_| CodecFrameError::Invalid)?;
    envelope
        .payload
        .validate()
        .map_err(|_| CodecFrameError::Invalid)?;
    if envelope.payload.declared_body_bytes().unwrap_or(0) != body.len() {
        return Err(CodecFrameError::Invalid);
    }
    Ok((envelope, body))
}

fn write_frame<W: Write, T: Serialize>(
    writer: &mut W,
    envelope: &T,
    body: &[u8],
) -> Result<(), CodecFrameError> {
    let header = serde_json::to_vec(envelope).map_err(CodecFrameError::Encode)?;
    if header.is_empty() || header.len() > MAX_CODEC_CONTROL_FRAME_BYTES {
        return Err(if header.is_empty() {
            CodecFrameError::Empty
        } else {
            CodecFrameError::TooLarge
        });
    }
    let header_len = u32::try_from(header.len()).map_err(|_| CodecFrameError::TooLarge)?;
    let body_len = u32::try_from(body.len()).map_err(|_| CodecFrameError::TooLarge)?;
    writer
        .write_all(&header_len.to_be_bytes())
        .and_then(|_| writer.write_all(&body_len.to_be_bytes()))
        .and_then(|_| writer.write_all(&header))
        .and_then(|_| writer.write_all(body))
        .and_then(|_| writer.flush())
        .map_err(CodecFrameError::Io)
}

fn read_frame<R: Read, T: DeserializeOwned>(
    reader: &mut R,
    maximum_body: usize,
) -> Result<(T, Vec<u8>), CodecFrameError> {
    let mut lengths = [0_u8; 8];
    match reader.read(&mut lengths[..1]) {
        Ok(0) => return Err(CodecFrameError::Closed),
        Ok(1) => {}
        Ok(_) => unreachable!("one-byte read returned more than one byte"),
        Err(error) => return Err(CodecFrameError::Io(error)),
    }
    reader
        .read_exact(&mut lengths[1..])
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::UnexpectedEof => CodecFrameError::TruncatedHeader,
            _ => CodecFrameError::Io(error),
        })?;
    let header_len = u32::from_be_bytes(lengths[..4].try_into().expect("fixed length")) as usize;
    let body_len = u32::from_be_bytes(lengths[4..].try_into().expect("fixed length")) as usize;
    if header_len == 0 {
        return Err(CodecFrameError::Empty);
    }
    if header_len > MAX_CODEC_CONTROL_FRAME_BYTES || body_len > maximum_body {
        return Err(CodecFrameError::TooLarge);
    }
    let mut header = vec![0_u8; header_len];
    let mut body = vec![0_u8; body_len];
    reader
        .read_exact(&mut header)
        .map_err(CodecFrameError::Io)?;
    reader.read_exact(&mut body).map_err(CodecFrameError::Io)?;
    let envelope = serde_json::from_slice(&header).map_err(CodecFrameError::Decode)?;
    Ok((envelope, body))
}

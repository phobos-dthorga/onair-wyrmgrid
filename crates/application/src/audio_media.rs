use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use wyrmgrid_storage::DatabaseKey;
use zeroize::{Zeroize, ZeroizeOnDrop};

const ENVELOPE_MAGIC: &[u8; 8] = b"WGAUDSEG";
pub const AUDIO_MEDIA_ENVELOPE_VERSION: u16 = 1;
pub const AUDIO_MEDIA_KEY_VERSION: u16 = 1;
const NONCE_BYTES: usize = 24;
const HEADER_BYTES: usize = 8 + 2 + 2 + NONCE_BYTES + 8;
const MAX_SEGMENT_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
const PACKET_STREAM_MAGIC: &[u8; 8] = b"WGAUPKT1";
const HKDF_SALT: &[u8] = b"WyrmGrid audio media HKDF salt v1";
const HKDF_INFO: &[u8] = b"dev.wyrmgrid.audio.media.xchacha20poly1305.v1";

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AudioMediaKey([u8; 32]);

impl AudioMediaKey {
    pub fn derive(database_key: &DatabaseKey) -> Result<Self, AudioMediaError> {
        let mut key = [0_u8; 32];
        Hkdf::<Sha256>::new(Some(HKDF_SALT), database_key.expose())
            .expand(HKDF_INFO, &mut key)
            .map_err(|_| AudioMediaError::KeyDerivation)?;
        Ok(Self(key))
    }

    #[cfg(test)]
    pub(crate) fn from_test_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Debug for AudioMediaKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AudioMediaKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedAudioPacket {
    pub sequence: u64,
    pub provider_monotonic_ns: u64,
    pub duration_48khz_frames: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSegmentContext {
    pub session_id: String,
    pub track_id: String,
    pub segment_index: u32,
    pub first_frame: u64,
    pub frame_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAudioSegment {
    pub storage_key: String,
    pub encrypted_bytes: u64,
    pub envelope_sha256: String,
    pub envelope_version: u16,
    pub key_version: u16,
}

#[derive(Debug, Error)]
pub enum AudioMediaError {
    #[error("the audio media encryption key could not be derived")]
    KeyDerivation,
    #[error("audio media randomness is unavailable")]
    RandomnessUnavailable,
    #[error("audio media input is invalid or outside supported bounds")]
    InvalidInput,
    #[error("audio media could not be written")]
    Write(#[source] std::io::Error),
    #[error("audio media could not be read")]
    Read(#[source] std::io::Error),
    #[error("audio media authentication failed")]
    AuthenticationFailed,
    #[error("audio media is damaged or uses an unsupported envelope")]
    InvalidEnvelope,
    #[error("audio media storage contains an unsafe redirected path")]
    UnsafeStoragePath,
}

#[derive(Clone)]
pub struct EncryptedAudioMediaStore {
    root: PathBuf,
    key: std::sync::Arc<AudioMediaKey>,
}

impl EncryptedAudioMediaStore {
    pub fn new(root: impl Into<PathBuf>, key: AudioMediaKey) -> Self {
        Self {
            root: root.into(),
            key: std::sync::Arc::new(key),
        }
    }

    pub fn write_segment(
        &self,
        context: &AudioSegmentContext,
        packets: &[EncodedAudioPacket],
    ) -> Result<StoredAudioSegment, AudioMediaError> {
        validate_context(context)?;
        let plaintext = encode_packets(packets)?;
        let storage_key = random_storage_key()?;
        let mut nonce = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| AudioMediaError::RandomnessUnavailable)?;
        let header = envelope_header(&nonce, plaintext.len())?;
        let aad = envelope_aad(&header, &storage_key, context)?;
        let cipher = XChaCha20Poly1305::new((&self.key.0).into());
        let nonce =
            XNonce::try_from(nonce.as_slice()).map_err(|_| AudioMediaError::InvalidInput)?;
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| AudioMediaError::AuthenticationFailed)?;
        let mut envelope = Vec::with_capacity(header.len() + ciphertext.len());
        envelope.extend_from_slice(&header);
        envelope.extend_from_slice(&ciphertext);

        let final_path = self.path_for_key(&storage_key)?;
        let pending_path = final_path.with_extension("pending");
        self.prepare_storage_directory(&storage_key)?;
        let write_result = (|| {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&pending_path)
                .map_err(AudioMediaError::Write)?;
            file.write_all(&envelope)
                .and_then(|_| file.sync_all())
                .map_err(AudioMediaError::Write)?;
            fs::rename(&pending_path, &final_path).map_err(AudioMediaError::Write)
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&pending_path);
        }
        write_result?;

        Ok(StoredAudioSegment {
            storage_key,
            encrypted_bytes: envelope.len() as u64,
            envelope_sha256: lower_hex(&Sha256::digest(&envelope)),
            envelope_version: AUDIO_MEDIA_ENVELOPE_VERSION,
            key_version: AUDIO_MEDIA_KEY_VERSION,
        })
    }

    pub fn read_segment(
        &self,
        storage_key: &str,
        expected_sha256: &str,
        context: &AudioSegmentContext,
    ) -> Result<Vec<EncodedAudioPacket>, AudioMediaError> {
        validate_context(context)?;
        if !valid_lower_hex(storage_key, 32) || !valid_lower_hex(expected_sha256, 64) {
            return Err(AudioMediaError::InvalidInput);
        }
        self.reject_redirected_storage_path(storage_key)?;
        let envelope = fs::read(self.path_for_key(storage_key)?).map_err(AudioMediaError::Read)?;
        if lower_hex(&Sha256::digest(&envelope)) != expected_sha256 {
            return Err(AudioMediaError::AuthenticationFailed);
        }
        if envelope.len() < HEADER_BYTES + 16 || &envelope[..8] != ENVELOPE_MAGIC {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        let version = u16::from_be_bytes(envelope[8..10].try_into().expect("fixed header"));
        let key_version = u16::from_be_bytes(envelope[10..12].try_into().expect("fixed header"));
        if version != AUDIO_MEDIA_ENVELOPE_VERSION || key_version != AUDIO_MEDIA_KEY_VERSION {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        let nonce = XNonce::try_from(&envelope[12..12 + NONCE_BYTES])
            .map_err(|_| AudioMediaError::InvalidEnvelope)?;
        let plaintext_length = u64::from_be_bytes(
            envelope[12 + NONCE_BYTES..HEADER_BYTES]
                .try_into()
                .expect("fixed header"),
        );
        if plaintext_length as usize > MAX_SEGMENT_PLAINTEXT_BYTES {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        let aad = envelope_aad(&envelope[..HEADER_BYTES], storage_key, context)?;
        let cipher = XChaCha20Poly1305::new((&self.key.0).into());
        let plaintext = cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &envelope[HEADER_BYTES..],
                    aad: &aad,
                },
            )
            .map_err(|_| AudioMediaError::AuthenticationFailed)?;
        if plaintext.len() != plaintext_length as usize {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        decode_packets(&plaintext)
    }

    pub fn delete_segment(&self, storage_key: &str) -> Result<(), AudioMediaError> {
        self.reject_redirected_storage_path(storage_key)?;
        let path = self.path_for_key(storage_key)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AudioMediaError::Write(error)),
        }
    }

    pub fn discard_pending_segments(&self) -> Result<usize, AudioMediaError> {
        self.cleanup_segments(&BTreeSet::new(), false)
    }

    pub fn discard_orphan_segments(
        &self,
        retained_storage_keys: &BTreeSet<String>,
    ) -> Result<usize, AudioMediaError> {
        self.cleanup_segments(retained_storage_keys, true)
    }

    fn cleanup_segments(
        &self,
        retained_storage_keys: &BTreeSet<String>,
        remove_unretained_complete: bool,
    ) -> Result<usize, AudioMediaError> {
        match fs::symlink_metadata(&self.root) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(AudioMediaError::UnsafeStoragePath);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(AudioMediaError::Read(error)),
        }
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(AudioMediaError::Read(error)),
        };
        let mut removed = 0;
        for entry in entries {
            let entry = entry.map_err(AudioMediaError::Read)?;
            if !entry.file_type().map_err(AudioMediaError::Read)?.is_dir() {
                continue;
            }
            let Some(prefix) = entry
                .file_name()
                .to_str()
                .filter(|prefix| valid_lower_hex(prefix, 2))
                .map(str::to_owned)
            else {
                continue;
            };
            for candidate in fs::read_dir(entry.path()).map_err(AudioMediaError::Read)? {
                let candidate = candidate.map_err(AudioMediaError::Read)?;
                let path = candidate.path();
                if !candidate
                    .file_type()
                    .map_err(AudioMediaError::Read)?
                    .is_file()
                {
                    continue;
                }
                let candidate_key = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .filter(|key| valid_lower_hex(key, 32) && key.starts_with(&prefix));
                let is_pending = candidate_key.is_some()
                    && path.extension().and_then(|extension| extension.to_str()) == Some("pending");
                let complete_key =
                    if path.extension().and_then(|extension| extension.to_str()) == Some("wga") {
                        candidate_key
                    } else {
                        None
                    };
                if is_pending
                    || (remove_unretained_complete
                        && complete_key.is_some_and(|key| !retained_storage_keys.contains(key)))
                {
                    fs::remove_file(path).map_err(AudioMediaError::Write)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    fn prepare_storage_directory(&self, storage_key: &str) -> Result<(), AudioMediaError> {
        if !valid_lower_hex(storage_key, 32) {
            return Err(AudioMediaError::InvalidInput);
        }
        match fs::symlink_metadata(&self.root) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(AudioMediaError::UnsafeStoragePath);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.root).map_err(AudioMediaError::Write)?;
            }
            Err(error) => return Err(AudioMediaError::Write(error)),
        }
        let prefix = self.root.join(&storage_key[..2]);
        match fs::symlink_metadata(&prefix) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(AudioMediaError::UnsafeStoragePath)
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&prefix).map_err(AudioMediaError::Write)
            }
            Err(error) => Err(AudioMediaError::Write(error)),
        }
    }

    fn reject_redirected_storage_path(&self, storage_key: &str) -> Result<(), AudioMediaError> {
        if !valid_lower_hex(storage_key, 32) {
            return Err(AudioMediaError::InvalidInput);
        }
        let prefix = self.root.join(&storage_key[..2]);
        for component in [&self.root, &prefix] {
            match fs::symlink_metadata(component) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(AudioMediaError::UnsafeStoragePath);
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AudioMediaError::Read(error)),
            }
        }
        Ok(())
    }

    fn path_for_key(&self, storage_key: &str) -> Result<PathBuf, AudioMediaError> {
        if !valid_lower_hex(storage_key, 32) {
            return Err(AudioMediaError::InvalidInput);
        }
        Ok(self
            .root
            .join(&storage_key[..2])
            .join(format!("{storage_key}.wga")))
    }
}

pub fn encode_packet_export(packets: &[EncodedAudioPacket]) -> Result<Vec<u8>, AudioMediaError> {
    encode_packets(packets)
}

fn encode_packets(packets: &[EncodedAudioPacket]) -> Result<Vec<u8>, AudioMediaError> {
    if packets.is_empty() || packets.len() > u32::MAX as usize {
        return Err(AudioMediaError::InvalidInput);
    }
    let mut output = Vec::new();
    output.extend_from_slice(PACKET_STREAM_MAGIC);
    output.extend_from_slice(&(packets.len() as u32).to_be_bytes());
    let mut previous_sequence = 0;
    for packet in packets {
        if packet.sequence <= previous_sequence
            || ![120, 240, 480, 960, 1_920, 2_880].contains(&packet.duration_48khz_frames)
            || packet.bytes.is_empty()
            || packet.bytes.len() > 16 * 1024
        {
            return Err(AudioMediaError::InvalidInput);
        }
        previous_sequence = packet.sequence;
        output.extend_from_slice(&packet.sequence.to_be_bytes());
        output.extend_from_slice(&packet.provider_monotonic_ns.to_be_bytes());
        output.extend_from_slice(&packet.duration_48khz_frames.to_be_bytes());
        output.extend_from_slice(&(packet.bytes.len() as u32).to_be_bytes());
        output.extend_from_slice(&packet.bytes);
        if output.len() > MAX_SEGMENT_PLAINTEXT_BYTES {
            return Err(AudioMediaError::InvalidInput);
        }
    }
    Ok(output)
}

fn decode_packets(input: &[u8]) -> Result<Vec<EncodedAudioPacket>, AudioMediaError> {
    if input.len() < 12 || &input[..8] != PACKET_STREAM_MAGIC {
        return Err(AudioMediaError::InvalidEnvelope);
    }
    let count = u32::from_be_bytes(input[8..12].try_into().expect("fixed header")) as usize;
    let mut offset = 12;
    let mut packets = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        if input.len().saturating_sub(offset) < 22 {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        let sequence = u64::from_be_bytes(input[offset..offset + 8].try_into().expect("bounded"));
        let provider_monotonic_ns =
            u64::from_be_bytes(input[offset + 8..offset + 16].try_into().expect("bounded"));
        let duration_48khz_frames =
            u16::from_be_bytes(input[offset + 16..offset + 18].try_into().expect("bounded"));
        let body_length =
            u32::from_be_bytes(input[offset + 18..offset + 22].try_into().expect("bounded"))
                as usize;
        offset += 22;
        if body_length == 0
            || body_length > 16 * 1024
            || input.len().saturating_sub(offset) < body_length
        {
            return Err(AudioMediaError::InvalidEnvelope);
        }
        packets.push(EncodedAudioPacket {
            sequence,
            provider_monotonic_ns,
            duration_48khz_frames,
            bytes: input[offset..offset + body_length].to_vec(),
        });
        offset += body_length;
    }
    if offset != input.len()
        || packets.is_empty()
        || packets
            .windows(2)
            .any(|pair| pair[0].sequence >= pair[1].sequence)
    {
        return Err(AudioMediaError::InvalidEnvelope);
    }
    Ok(packets)
}

fn envelope_header(
    nonce: &[u8; NONCE_BYTES],
    plaintext_length: usize,
) -> Result<Vec<u8>, AudioMediaError> {
    let plaintext_length =
        u64::try_from(plaintext_length).map_err(|_| AudioMediaError::InvalidInput)?;
    let mut header = Vec::with_capacity(HEADER_BYTES);
    header.extend_from_slice(ENVELOPE_MAGIC);
    header.extend_from_slice(&AUDIO_MEDIA_ENVELOPE_VERSION.to_be_bytes());
    header.extend_from_slice(&AUDIO_MEDIA_KEY_VERSION.to_be_bytes());
    header.extend_from_slice(nonce);
    header.extend_from_slice(&plaintext_length.to_be_bytes());
    Ok(header)
}

fn envelope_aad(
    header: &[u8],
    storage_key: &str,
    context: &AudioSegmentContext,
) -> Result<Vec<u8>, AudioMediaError> {
    let mut aad = Vec::new();
    aad.extend_from_slice(header);
    push_bounded_text(&mut aad, storage_key, 32)?;
    push_bounded_text(&mut aad, &context.session_id, 128)?;
    push_bounded_text(&mut aad, &context.track_id, 128)?;
    aad.extend_from_slice(&context.segment_index.to_be_bytes());
    aad.extend_from_slice(&context.first_frame.to_be_bytes());
    aad.extend_from_slice(&context.frame_count.to_be_bytes());
    Ok(aad)
}

fn push_bounded_text(
    output: &mut Vec<u8>,
    value: &str,
    maximum: usize,
) -> Result<(), AudioMediaError> {
    if value.is_empty() || value.len() > maximum {
        return Err(AudioMediaError::InvalidInput);
    }
    let length = u16::try_from(value.len()).map_err(|_| AudioMediaError::InvalidInput)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn validate_context(context: &AudioSegmentContext) -> Result<(), AudioMediaError> {
    if context.session_id.is_empty()
        || context.session_id.len() > 128
        || context.track_id.is_empty()
        || context.track_id.len() > 128
        || context.frame_count == 0
    {
        return Err(AudioMediaError::InvalidInput);
    }
    Ok(())
}

fn random_storage_key() -> Result<String, AudioMediaError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| AudioMediaError::RandomnessUnavailable)?;
    Ok(lower_hex(&bytes))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn audio_media_root(app_data_directory: &Path) -> PathBuf {
    app_data_directory.join("audio-media-v1")
}

#[cfg(test)]
#[path = "tests/audio_media.rs"]
mod tests;

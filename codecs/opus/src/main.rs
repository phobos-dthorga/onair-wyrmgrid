use opus2::{Application, Bitrate, Channels, Encoder};
use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter, Write, stdin, stdout};
use wyrmgrid_audio_codec_protocol::{
    AudioCodecDescriptor, AudioCodecHostMessage, AudioCodecPlatform, AudioCodecProfile,
    AudioCodecProviderMessage, AudioCodecState, CodecEnvelope, MAX_CODEC_PACKET_BYTES,
    read_codec_host_frame, validate_next_codec_sequence, write_codec_provider_frame,
};
use wyrmgrid_domain::AudioProfileId;

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let mut input = BufReader::new(stdin().lock());
    let mut output = CodecWriter::new(BufWriter::new(stdout().lock()));
    let mut incoming_sequence = 0;
    let mut tracks = BTreeMap::<(String, String), OpusTrack>::new();

    loop {
        let (envelope, body) = read_codec_host_frame(&mut input).map_err(|_| ())?;
        if !validate_next_codec_sequence(incoming_sequence, envelope.sequence) {
            return Err(());
        }
        incoming_sequence = envelope.sequence;
        match envelope.payload {
            AudioCodecHostMessage::Hello {
                codec_provider_id, ..
            } if codec_provider_id == "dev.wyrmgrid.opus" => {
                output.send(
                    AudioCodecProviderMessage::State {
                        state: AudioCodecState::Starting,
                        code: "codec.starting".into(),
                    },
                    &[],
                )?;
                output.send(
                    AudioCodecProviderMessage::Hello {
                        codec: descriptor(),
                    },
                    &[],
                )?;
                output.send(
                    AudioCodecProviderMessage::State {
                        state: AudioCodecState::Ready,
                        code: "codec.ready".into(),
                    },
                    &[],
                )?;
            }
            AudioCodecHostMessage::StartTrack {
                session_id,
                track_id,
                profile,
            } => {
                let key = (session_id.clone(), track_id.clone());
                if tracks.contains_key(&key) {
                    output.error("codec.track_already_started")?;
                    continue;
                }
                let track = OpusTrack::new(profile).map_err(|_| ())?;
                tracks.insert(key, track);
                output.send(
                    AudioCodecProviderMessage::TrackStarted {
                        session_id,
                        track_id,
                        profile,
                    },
                    &[],
                )?;
            }
            AudioCodecHostMessage::EncodePcm {
                session_id,
                track_id,
                frame_sequence,
                provider_monotonic_ns,
                channels,
                sample_rate_hz,
                frame_count,
                ..
            } => {
                let key = (session_id.clone(), track_id.clone());
                let Some(track) = tracks.get_mut(&key) else {
                    output.error("codec.track_not_started")?;
                    continue;
                };
                if channels != track.profile().spec().channels
                    || sample_rate_hz != track.profile().spec().sample_rate_hz
                    || frame_count != 960
                    || frame_sequence != track.next_sequence
                {
                    output.error("codec.invalid_pcm_frame")?;
                    continue;
                }
                let samples = decode_s16le(&body).ok_or(())?;
                let packet = track.encode(&samples).map_err(|_| ())?;
                output.send(
                    AudioCodecProviderMessage::EncodedPacket {
                        session_id,
                        track_id,
                        packet_sequence: frame_sequence,
                        provider_monotonic_ns,
                        duration_48khz_frames: frame_count,
                        payload_bytes: packet.len() as u32,
                    },
                    &packet,
                )?;
                track.next_sequence += 1;
            }
            AudioCodecHostMessage::StopTrack {
                session_id,
                track_id,
            } => {
                tracks.remove(&(session_id.clone(), track_id.clone()));
                output.send(
                    AudioCodecProviderMessage::TrackStopped {
                        session_id,
                        track_id,
                    },
                    &[],
                )?;
            }
            AudioCodecHostMessage::Shutdown => {
                output.send(
                    AudioCodecProviderMessage::State {
                        state: AudioCodecState::Stopped,
                        code: "codec.stopped".into(),
                    },
                    &[],
                )?;
                return Ok(());
            }
            AudioCodecHostMessage::Hello { .. } => return Err(()),
        }
    }
}

struct OpusTrack {
    profile: AudioProfileId,
    encoder: Encoder,
    next_sequence: u64,
}

impl OpusTrack {
    fn new(profile: AudioProfileId) -> Result<Self, opus2::Error> {
        let spec = profile.spec();
        let channels = if spec.channels == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };
        let application = if profile == AudioProfileId::MixedStereoV1 {
            Application::Audio
        } else {
            Application::Voip
        };
        let mut encoder = Encoder::new(spec.sample_rate_hz, channels, application)?;
        encoder.set_bitrate(Bitrate::Bits(target_bitrate(profile) as i32))?;
        encoder.set_vbr(true)?;
        Ok(Self {
            profile,
            encoder,
            next_sequence: 1,
        })
    }

    fn profile(&self) -> AudioProfileId {
        self.profile
    }

    fn encode(&mut self, samples: &[i16]) -> Result<Vec<u8>, opus2::Error> {
        self.encoder.encode_vec(samples, MAX_CODEC_PACKET_BYTES)
    }
}

fn decode_s16le(bytes: &[u8]) -> Option<Vec<i16>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    Some(
        bytes
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect(),
    )
}

fn target_bitrate(profile: AudioProfileId) -> u32 {
    match profile {
        AudioProfileId::PilotMicrophoneV1 => 48_000,
        AudioProfileId::IsolatedVoiceV1 => 32_000,
        AudioProfileId::MixedStereoV1 => 128_000,
    }
}

fn profiles() -> Vec<AudioCodecProfile> {
    [
        AudioProfileId::PilotMicrophoneV1,
        AudioProfileId::IsolatedVoiceV1,
        AudioProfileId::MixedStereoV1,
    ]
    .into_iter()
    .map(|id| AudioCodecProfile {
        id,
        codec_id: "opus".into(),
        media_type: "audio/opus".into(),
        channels: id.spec().channels,
        sample_rate_hz: id.spec().sample_rate_hz,
        target_bitrate_bps: target_bitrate(id),
        packet_duration_48khz_frames: 960,
    })
    .collect()
}

fn descriptor() -> AudioCodecDescriptor {
    AudioCodecDescriptor {
        id: "dev.wyrmgrid.opus".into(),
        name: "WyrmGrid Opus codec".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: current_platform(),
        profiles: profiles(),
    }
}

fn current_platform() -> AudioCodecPlatform {
    #[cfg(target_os = "windows")]
    return AudioCodecPlatform::WindowsX86_64;
    #[cfg(target_os = "linux")]
    return AudioCodecPlatform::LinuxX86_64;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return AudioCodecPlatform::MacosAarch64;
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    return AudioCodecPlatform::MacosX86_64;
}

struct CodecWriter<W: Write> {
    writer: W,
    sequence: u64,
}

impl<W: Write> CodecWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            sequence: 0,
        }
    }

    fn send(&mut self, message: AudioCodecProviderMessage, body: &[u8]) -> Result<(), ()> {
        self.sequence = self.sequence.checked_add(1).ok_or(())?;
        write_codec_provider_frame(
            &mut self.writer,
            &CodecEnvelope::new(self.sequence, message),
            body,
        )
        .map_err(|_| ())
    }

    fn error(&mut self, code: &str) -> Result<(), ()> {
        self.send(AudioCodecProviderMessage::Error { code: code.into() }, &[])
    }
}

# Audio Codec Provider protocol version 1

Status: protocol, explicit development registration, and first-party Opus
provider implemented; community installation and packaging not implemented

Audio Codec Providers let an end user choose the encoder for each approved
source without coupling that choice to Windows, macOS, Linux, MSFS, or X-Plane
capture code. They are specialised supervised sidecars, not in-process native
libraries and not recipients of the general plugin audio capability.

## Manifest and selection

A version-1 manifest declares a reverse-domain provider ID, safe relative entry
point, supported platforms, `encode_pcm_s16le`, and one to sixteen profiles.
Each profile binds a stable role ID to its codec ID, media type, channel count,
48 kHz sample rate, target bitrate, and packet duration.

WyrmGrid displays only codecs compatible with a source profile. The selected
provider ID is stored with the source selection. Each new track snapshots the
provider ID and version plus the negotiated codec ID and media type so later
provider updates cannot reinterpret recorded packets. Missing, incompatible,
duplicated, malformed, or unavailable providers fail closed; WyrmGrid never
silently chooses a replacement codec.

## Framing and lifecycle

Frames use 32-bit big-endian JSON and binary lengths. JSON and PCM are each
limited to 64 KiB; encoded packets are limited to 16 KiB. PCM is signed 16-bit
little-endian, interleaved, exactly 48 kHz, and must exactly match the declared
channel and frame counts. At most eight tracks and sixteen profiles are
accepted.

The host validates a three-message startup (`hello`, `manifest`, `ready`), then
sends `start_track`, ordered `encode_pcm` frames, `stop_track`, and `shutdown`.
The provider acknowledges track lifecycle and returns an encoded packet whose
session, track, sequence, monotonic time, duration, and body length must match
the negotiated state. Codec ID and media type come from the exact manifest
profile snapshotted when the track starts. Unknown or non-increasing messages,
timeouts, unexpected bodies, and length mismatches terminate the process
session.

## Privacy and security

A selected codec necessarily receives the selected audio content as transient
PCM. It receives no source label or OS device identity, microphone permission,
OnAir key, database key, XChaCha media key, storage path, plaintext export
destination, retention authority, Sentry channel, optional-AI channel, or
network capability from this contract. WyrmGrid encrypts returned packets and
persists only encoded segments.

Future community support must add verified installation roots, publisher and
package identity, signing and integrity, explicit trust presentation, version
resolution, updates and rollback, process resource limits, removal, and a clear
privacy disclosure before arbitrary codec executables may be registered.

## First-party Opus provider

`dev.wyrmgrid.opus` is implemented as a normal provider using the pure-Rust
`opus2` Mousiki backend. It supports the microphone, isolated-voice, and mixed-
stereo profiles with 20 ms packets. A black-box test starts the executable,
encodes synthetic zero PCM, decodes the packet, and verifies its duration. This
is integration evidence only; it does not claim independent Opus conformance,
audio quality, security certification, or real-device support.

Schemas and sanitized fixtures:

- `schemas/audio-codec-manifest-v1.schema.json`
- `schemas/audio-codec-envelope-v1.schema.json`
- host hello, start-track, PCM-header/body, and encoded-header/body fixtures in
  `schemas/fixtures/`

Protocol version 1 is new and makes no application semantic-version, Bridge,
general plugin, portable-backup, or installer compatibility change.

# Audio Codec Provider protocol version 1

Status: protocol, managed `.wyrmcodec` packaging and lifecycle, first-party
Opus package, and synthetic end-to-end grounding implemented; publisher
signing, OS resource isolation, and live certification remain release gates

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

Community packages use a verified host-owned installation root, exact package
identity and SHA-256 inventory, explicit unverified-native-code trust
presentation, immutable versions, one-step rollback, disable, and tombstoned
removal. Installation never starts a codec or grants recording consent. Active
recording blocks codec mutation. Publisher signing, authenticated updates,
revocation, OS-enforced process resource limits, and the release privacy/legal
disclosure remain future gates; local integrity is not publisher trust.

## Package and managed lifecycle

Audio Codec package schema version 1 uses the `.wyrmcodec` suffix and
`audio_codec_provider` package kind. Offline inspection exposes the declared
ID, version, author, codec protocol, platforms, capabilities, and profiles
before the user accepts the native-code warning. Installation stages and
validates the exact inventory under a canonical per-user root. Only the enabled
active version for the current platform enters codec discovery.

An unavailable, disabled, removed, malformed, or incompatible selected codec
fails closed. WyrmGrid does not rewrite the saved source choice or silently
substitute another encoder. See
[Authoring external WyrmGrid extensions](extension-authoring.md) for the
scaffolder, packager, compatibility rules, and community test checklist.

## First-party Opus provider

`dev.wyrmgrid.opus` is implemented and seeded as a normal `.wyrmcodec` provider
using the pure-Rust
`opus2` Mousiki backend. It supports the microphone, isolated-voice, and mixed-
stereo profiles with 20 ms packets. A black-box test starts the executable,
encodes synthetic zero PCM, decodes the packet, and verifies its duration. This
is integration evidence only; it does not claim independent Opus conformance,
audio quality, security certification, or real-device support.

On Windows, a second application test installs the separately distributable
deterministic `.wyrmaudio` and Opus `.wyrmcodec` artifacts, performs explicit
permission and source selection, captures synthetic PCM, encodes it through the
managed Opus process, stores it in authenticated encrypted media, and reads it
back with exact codec provenance.

Schemas and sanitized fixtures:

- `schemas/audio-codec-manifest-v1.schema.json`
- `schemas/audio-codec-package-manifest-v1.schema.json`
- `schemas/audio-codec-envelope-v1.schema.json`
- host hello, start-track, PCM-header/body, and encoded-header/body fixtures in
  `schemas/fixtures/`

Protocol version 1 is new and makes no application semantic-version, Bridge,
general plugin, portable-backup, or installer compatibility change.

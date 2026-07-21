# ADR-0020: Out-of-process audio codec providers

Status: accepted

## Context

ADR-0017 selected Opus as WyrmGrid's first working audio codec and assigned
capture and encoding to one native provider. That couples operating-system
device access to a particular media format and prevents an end user from
choosing another codec without replacing the capture implementation.

Audio codecs also process private, attacker-influenced binary data. Loading
community codec libraries into the desktop process would introduce native ABI,
memory-safety, crash, dependency, and update coupling that conflicts with the
project's out-of-process extension boundary.

## Decision

Capture and encoding are separate supervised sidecars:

```text
approved source -> Audio Capture Provider v2 -> bounded PCM
               -> selected Audio Codec Provider v1 -> encoded packets
               -> WyrmGrid encryption and storage
```

- Audio Capture Provider version 2 emits signed 16-bit little-endian PCM at the
  fixed 48 kHz working rate. It does not select a codec, encrypt media, or know
  storage paths.
- Audio Codec Provider version 1 accepts only host-selected, bounded PCM frames
  and returns bounded encoded packets. It receives no microphone authority,
  device identifier, encryption key, database handle, or media path.
- The first-party `dev.wyrmgrid.opus` implementation is a codec provider using
  exactly the same protocol as a future end-user codec. It receives no private
  in-process exception.
- Stable profile identifiers describe recording roles and channel shape. Each
  codec manifest declares which profiles it supports plus its codec identifier,
  media type, bitrate, sample rate, channels, and packet duration.
- The user explicitly chooses a compatible codec for each selected source.
  WyrmGrid records the selected provider on the preference, then snapshots its
  provider identity and version plus codec ID and media type on every recorded
  track. A missing or incompatible codec fails closed without changing source
  or format silently.
- WyrmGrid alone owns consent, capture lifecycle, codec selection, validation,
  XChaCha20-Poly1305 encryption, metadata, retention, deletion, playback
  inspection, and export.
- Codec-provider installation, signing, publisher identity, discovery, updates,
  rollback, resource quotas, and community trust presentation remain separate
  release gates. The implemented development host uses explicit registrations;
  it does not yet accept arbitrary executables from a plugin directory.

This is a specialised codec-provider contract rather than a new capability in
the general community plugin protocol. General plugins remain denied audio.

## Compatibility

Audio Capture Provider version 1 is archived. It carried encoded packets and
was never released with native capture, so the host does not implement a v1-to-
v2 adapter. Its schemas and fixtures remain as historical compatibility
evidence. Version 2 is intentionally incompatible and requires a version-2
manifest.

Audio Codec Provider version 1 is new. Database migration 20 adds the selected
provider ID to source selections and adds provider ID/version plus codec
ID/media type provenance to tracks. Existing schema-19 rows receive
`dev.wyrmgrid.opus`, `legacy-unversioned`, `opus`, and `audio/opus`, preserving
the only format implied by the earlier design without inventing a historical
provider version. The application semantic version, Bridge protocol, community
plugin protocol, portable-backup format, and installer identity do not change.

## Consequences

The extra process and protocol add bounded orchestration work, but remove a
larger long-term coupling between every platform capture provider and every
codec. Codec crashes are isolated from the desktop and capture process; a
failure interrupts the affected recording while telemetry continues.

Raw PCM exists transiently in bounded host and sidecar buffers. It is never
persisted by WyrmGrid, placed in SQLite, logged, exported, sent to Sentry,
exposed to general plugins, or included in optional-AI handoffs. A selected
third-party codec necessarily receives the selected audio content, so future
community installation must present that privacy consequence before use.

The first-party Opus sidecar currently uses the pure-Rust `opus2` Mousiki
backend. Deterministic encode/decode tests establish protocol integration, not
independent codec conformance, quality, security certification, or live-device
support.

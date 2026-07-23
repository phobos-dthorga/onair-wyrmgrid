# ADR-0025: Audio codec provider package format version 1

- Status: Accepted
- Date: 2026-07-24

## Context

Audio Codec Provider protocol version 1 already isolates encoders in supervised,
language-neutral processes, but the desktop previously registered the
first-party Opus executable only from a debug build path. Community authors
could implement the protocol but could not deliver a codec independently from a
WyrmGrid compilation. ADR-0021 requires every provider to have an external,
replaceable artifact boundary.

A codec receives transient PCM for a source the user explicitly selected. It
does not need device identity, capture permission, retention authority,
credentials, simulator telemetry, media keys, storage paths, or network access.
Installation and enablement must remain separate from source and profile
selection.

## Decision

Audio codec package schema version 1 uses a ZIP envelope with the `.wyrmcodec`
extension and media type
`application/vnd.wyrmgrid.audio-codec-package+zip`.

The root `wyrmgrid-package.json` conforms to
`schemas/audio-codec-package-manifest-v1.schema.json` and declares:

- package schema version `1` and kind `audio_codec_provider`;
- the same reverse-domain identifier and three-part semantic version as the
  enclosed codec manifest;
- the fixed manifest path `audio-codec.json`; and
- the exact byte length and lowercase SHA-256 digest of every payload file.

The codec manifest remains authoritative for display metadata, the portable
entry-point stem, supported platforms, Codec Provider protocol version,
capabilities, and supported profiles. Every declared non-Windows platform
requires the exact entry-point path; Windows x86-64 requires the same path with
`.exe` appended. Package and codec identities and versions must match.

Codec packages reuse the common bounded-archive, exact-inventory, canonical
path, traversal/link rejection, immutable `(kind, id, version)`, staged
extraction, provenance, enable/disable, update, rollback, interrupted-removal
recovery, and tombstoned-removal rules. Owner-only execution is granted to the
current-platform entry point on Unix.

Installation first presents claimed identity, author, version, platform,
protocol, profiles, file count, size, and archive digest. Version 1 has no
publisher signature, so publisher verification is false and the interface
warns that the package contains unverified native code.

Installation does not launch the codec or change any source selection. Only an
enabled, current-platform package is offered for compatible profiles. A source
continues to name its exact selected codec provider; WyrmGrid never silently
substitutes another provider. Disabling or removing a codec makes that choice
unavailable while preserving the stored choice and historical track
provenance. Re-enabling is an explicit lifecycle action.

An active audio recording blocks codec install, disable, update, rollback, and
removal. Each newly started track snapshots provider ID and version, codec ID,
and media type. The managed executable launches only when needed with a scrubbed
environment and must complete the protocol handshake against its installed
manifest. Existing protocol frame, packet, profile, track, and timeout bounds
remain the first process-resource controls.

The first-party Opus encoder is built into a normal `opus.wyrmcodec` artifact,
seeded through the same public lifecycle, and independently distributable.
First-party installer inclusion is a convenience rather than a private
registration path.

## Compatibility

This adds package schema version 1 without changing Audio Codec Provider
protocol version 1, codec manifest schema version 1, the application semantic
version, source selection schema, or audio-track schema. The application
database advances to schema 22 solely to extend the constrained schema-21
extension-package kind catalogue with `audio_codec_provider`; the append-only
migration transactionally preserves every existing package version,
active/rollback pointer, enabled state, and foreign-key relationship. Existing
source and track records already store codec-provider identity and need no
rewrite. Unknown package schemas, kinds, manifest schemas, platforms,
protocols, capabilities, or profiles remain inert.

A platform-dependent payload map, signature, authenticated update feed,
revocation contract, pre-launch full payload re-hash, operating-system sandbox,
or CPU/memory quota requires a separate compatibility and security decision.

## Consequences

Audio codec providers can be inspected, installed, enabled, disabled, updated,
rolled back, and removed without rebuilding or relinking WyrmGrid. The
first-party encoder now proves the same public lifecycle offered to community
authors.

Exact inventory validation proves content consistency at installation, not
publisher identity or native-code safety. Local packages remain intentionally
labelled unverified. Recommendation through Aerie and live support remain gated
on signing, authenticated updates, stronger launch integrity and resource
controls, vulnerability handling, privacy review, and platform testing.

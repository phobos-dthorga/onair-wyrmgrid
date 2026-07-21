# ADR-0023: Audio provider package format version 1

- Status: Accepted
- Date: 2026-07-21

## Context

Audio Capture Provider protocol version 1 already defines a supervised,
language-neutral process boundary, but the desktop previously accepted only one
development provider injected at startup. A community author could implement
the protocol yet could not give users an independently installable artifact.
ADR-0020 requires audio providers to use the same external delivery principle
as ordinary plugins and simulator providers.

An audio provider is native executable code with ambient user rights and may
request access to microphones, output endpoints, application audio, or a
simulator mix. Package installation therefore needs explicit native-code trust
presentation and must remain separate from provider selection, operating-system
permission, WyrmGrid recording consent, source selection, and capture start.

## Decision

Audio provider package schema version 1 uses a ZIP envelope with the
`.wyrmaudio` extension and media type
`application/vnd.wyrmgrid.audio-provider-package+zip`.

The root `wyrmgrid-package.json` conforms to
`schemas/audio-provider-package-manifest-v1.schema.json` and declares:

- package schema version `1` and kind `audio_provider`;
- the same reverse-domain identifier and three-part semantic version as the
  enclosed audio-provider manifest;
- the fixed manifest path `audio-provider.json`; and
- the exact byte length and lowercase SHA-256 digest of every payload file.

The audio-provider manifest remains authoritative for name, author, portable
entry-point stem, supported platform and architecture, Audio Capture Provider
protocol version, and capabilities. Every declared non-Windows platform requires
that exact inventoried path; Windows x86-64 requires the same path with `.exe`
appended. The package and provider identities and versions must match.

Audio packages reuse the bounded archive, canonical-path, compression,
exact-inventory, traversal/link rejection, immutable `(kind, id, version)`,
staged extraction, provenance, interrupted-removal recovery, disable, update,
single-step rollback, and tombstoned-removal rules established by ADR-0021 and
ADR-0022. On Unix, managed extraction grants owner-only execution to the
declared entry point and no other payload.

Installation is a two-step local operation: WyrmGrid displays the claimed
identity, author, version, supported platforms, protocol version, capabilities,
file count, expanded size, and archive digest before asking for confirmation.
Package schema version 1 has no signature, so publisher verification is always
false and the interface warns that the archive contains unverified native code.

Installation does not select or launch the provider, request an operating-
system permission, enable audio consent, select a source, or start capture.
Enabled and selected are separate states. Selection is persisted separately
from recording preferences and resolves only an enabled managed package on a
supported platform. Disabling or removing the selected provider clears the
selection so later re-enabling or reinstalling the same ID cannot silently
restore capture authority.

An active audio recording prevents selection, disable, update, rollback, or
removal. Outside a recording, changing the selected package drops the supervised
process and clears source, level, and provider-runtime state. The next explicit
source operation launches the selected executable with a scrubbed environment.
Its protocol hello must match the installed manifest's ID, name, version,
platform, and complete capability list.

The deterministic fake provider is the first reference package. It is built as
`assets/audio-provider-packages/deterministic-fake-audio.wyrmaudio` through the
public packager and managed lifecycle, but remains a synthetic conformance tool,
is not seeded by the installer, and does not establish native capture or live
audio availability.

## Compatibility

Audio package schema version, audio-provider manifest schema version, Audio
Capture Provider protocol version, database migration version, and application
semantic version are independent contracts. This decision adds package schema
version 1 without changing audio protocol version 1 or audio-provider manifest
schema version 1. Unknown package schemas, kinds, manifest schemas, platforms,
protocol versions, compression methods, or capabilities remain inert.

One package can inventory both the portable entry point and its Windows `.exe`
counterpart when the manifest declares several platforms. A more general
platform-dependent payload map, native library loading contract, signature,
update feed, or sandbox field requires an explicit compatibility and security
decision.

## Consequences

Audio providers can be inspected, installed, selected, disabled, updated,
rolled back, and removed without rebuilding or relinking WyrmGrid. The same
public path can later carry first-party Windows, macOS, or Linux capture
providers and community integrations.

Exact archive validation proves content consistency, not publisher identity or
code safety. Version 1 has no signature, revocation, authenticated update,
pre-launch re-hash, operating-system sandbox, or CPU/memory quota. Recommending
unreviewed providers and claiming live capture remain gated on those controls,
native implementation evidence, platform permissions, privacy/legal review,
and real-device testing.

This decision implements the audio-provider portion of
[ADR-0020](0020-externally-installable-extensions.md), reuses the package
envelope decisions in [ADR-0021](0021-ordinary-plugin-package-format-v1.md) and
[ADR-0022](0022-simulator-provider-package-format-v1.md), and preserves the
separate consent boundary in
[ADR-0017](0017-simulator-synchronised-audio-recording.md).

# Audio Capture Provider protocol version 2

Status: protocol, external package lifecycle, deterministic fake provider, and
Windows microphone provider implemented; not installer-seeded or live-certified

Audio Capture Provider version 2 is the supervised boundary between an exact,
explicitly approved audio source and WyrmGrid. It enumerates capabilities,
handles an explicit operating-system permission request, captures PCM, reports
levels and discontinuities, and stops on command. It neither encodes nor stores
media.

## Compatibility decision

Version 1 transported encoded packets and made the capture provider choose
Opus. It was implemented only with a synthetic provider and never shipped with
native capture. Version 2 deliberately replaces that model with PCM capture so
end users can select a separate codec provider. The host accepts version 2
only; v1 schemas and fixtures remain unchanged as historical evidence.

This internal protocol change does not change application semantic version
0.3.1, Bridge protocol 1, community plugin protocol 1, portable-backup format
1, or installer identity. Any change to v2 message fields, framing,
interpretation, or enum values requires another explicit compatibility
decision.

Application schema 21 and English source catalogue 21 add managed package
state, persistent capture-provider selection, codec provenance, and interface
wording without changing those public compatibility markers.

## Process and authority boundary

Starting a provider does not authorize capture. WyrmGrid applies default-off
audio consent, an exact source selection, and any explicit OS permission action
before `start_capture`. Automatic telemetry recording cannot prompt for or
enable a microphone.

The provider receives source and track identifiers plus a codec-neutral profile
shape. It receives no OnAir credential, database key, audio media key, storage
path, retention rule, export authority, general plugin capability, or network
authority. Source labels and raw OS identifiers are private and excluded from
logs and optional-AI packets.

## Framing and limits

Both directions use a 32-bit big-endian JSON-header length. Provider frames add
a 32-bit big-endian binary-body length. JSON headers and PCM bodies are each
limited to 64 KiB and their declared and actual lengths must match before
allocation or interpretation.

## Package lifecycle

Package format version 1 now establishes deliberate local community-provider
installation through bounded `.wyrmaudio` archives, canonical managed paths,
staged validation, explicit native-code trust review, update, rollback,
disable/removal, and persistent selection. Installation grants no recording,
source, operating-system permission, OnAir, simulator, plugin, or network
authority. Publisher identity, signing, revocation, sandboxing, and Aerie
recommendation remain separate hardening work. See
[audio provider authoring](audio-provider-authoring.md) and
[ADR-0024](../architecture/decisions/0024-audio-provider-package-format-v1.md).

## PCM limits

PCM frames are signed 16-bit little-endian, interleaved, exactly 48 kHz, one of
the contract's bounded durations (120–2,880 frames per channel), and compatible
with the selected profile's channel count. At most 32 sources, eight tracks,
and 64 frames per drain request are accepted. Identifiers, labels, events,
level values, drift, and affected-frame counts are separately bounded.

Unknown fields or enum values, invalid UTF-8, unsupported versions, unsafe
manifest entry points, non-increasing sequences, unexpected bodies, mismatched
session/track identities, truncated frames, and oversized lengths fail closed.

## Lifecycle

1. Host sends `hello` for one expected provider identity.
2. Provider returns `hello` and bounded state transitions.
3. Host enumerates sources.
4. Only after an explicit user action, host may request permission for one
   source and receives a revised source list.
5. Host may exchange monotonic clock anchors.
6. Host starts one to eight exact source/track/profile requests.
7. Host repeatedly requests a bounded drain; provider responds with zero or
   more PCM frames, levels, events, then `drain_complete`.
8. Host stops capture or shuts the sidecar down.

No provider may silently fall back to another device. Gaps, dropouts,
backpressure, drift, source changes, permission problems, and failures remain
explicit facts.

## Implementations and evidence

`wyrmgrid-fake-audio-provider` supplies synthetic PCM and deterministic events
for hardware-independent tests. `wyrmgrid-windows-audio-provider` uses CPAL's
WASAPI host to enumerate and capture an explicitly selected microphone, hashes
raw OS device identifiers before they become source IDs, downmixes an approved
48 kHz stream to bounded mono PCM, never blocks the realtime callback, and
reports dropped frames as backpressure.

The Windows provider uses the same separately installable package boundary and
is not automatically selected or opened by tests. Its implementation does not
establish released or live microphone availability. Output, process-loopback,
MSFS, and X-Plane capture remain unimplemented.

Schemas and sanitized fixtures:

- `schemas/audio-provider-manifest-v2.schema.json`
- `schemas/audio-provider-package-manifest-v1.schema.json`
- `schemas/audio-provider-envelope-v2.schema.json`
- v2 hello, permission, sources, clock, PCM-header/body, and event fixtures in
  `schemas/fixtures/`
- retained v1 manifest/envelope schemas and fixtures

Tests validate framing, exact body sizes, identities, sequences, lifecycle,
unavailability, permission separation, synthetic PCM, callback conversion, and
bounded backpressure without opening real hardware.

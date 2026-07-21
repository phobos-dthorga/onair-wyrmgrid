# Audio Capture Provider protocol version 1

Status: protocol, external package lifecycle, and non-native application services implemented; no native capture or live availability

The Audio Capture Provider protocol is the supervised, language-neutral boundary
for future microphone, application-output, endpoint-output, simulator-mix, and
proven isolated-radio sources. It is independent of WyrmGrid Bridge version 1
and the community plugin protocol. Neither existing contract carries audio
media or receives an audio capability.

The implementation consists of the `wyrmgrid-audio-provider-protocol` crate,
stable source and Opus-profile models in `wyrmgrid-domain`, version-one schemas
and sanitized fixtures, `.wyrmaudio` package schema and managed lifecycle, a
deterministic development-only fake provider, and application services for default-off consent, provider orchestration,
encrypted packet segments, lifecycle policy, and authenticated packet
inspection/export. There is no native device access, audible decoding,
installer-seeded provider or live-support claim.

## Compatibility decision

Audio provider protocol version 1 is a new contract with no predecessor to
preserve. It does not change the application semantic version, Bridge protocol
version 1, plugin protocol version 1, simulator telemetry schemas, database
migrations, existing provider manifests, or installer identity.

Application schema 20 and English source catalogue 21 implement managed package
state, provider selection, and interface wording without changing audio provider protocol version 1,
Bridge protocol version 1, plugin protocol version 1, portable-backup format
version 1, application version 0.3.1, or installer identity.

Version-one JSON rejects unknown fields and unknown enum values. Adding or
removing a message, field, enum value, framing rule, or interpretation therefore
requires an explicit audio-protocol compatibility decision and normally a new
protocol version. Manifest-schema changes are separately versioned. Stable
machine-readable errors do not include source labels or encoded bytes.

## Process boundary

A future native provider is a separately supervised sidecar. Starting that
process does not authorize capture. The implemented application service selects
exact sources and profiles only after the separate default-off consent and
explicit permission rules are satisfied. The provider reports capabilities and facts; it does not choose
retention, fallback devices, storage, disclosure, or user policy.

The development-only `wyrmgrid-fake-audio-provider` is a protocol test tool. It
is a workspace member so black-box tests can launch it, but the Tauri external-
binary preparation and installer do not stage it. Its labels, identities,
timestamps, events, and packet bytes are synthetic.

Package format version 1 now establishes deliberate local community-provider
installation through bounded `.wyrmaudio` archives, canonical managed paths,
staged validation, explicit native-code trust review, update, rollback,
disable/removal, and persistent selection. Installation grants no recording,
source, operating-system permission, OnAir, simulator, plugin, or network
authority. Publisher identity, signing, revocation, sandboxing, and Aerie
recommendation remain separate hardening work. See
[audio provider authoring](audio-provider-authoring.md) and
[ADR-0023](../architecture/decisions/0023-audio-provider-package-format-v1.md).

## Framing

All integer frame lengths use unsigned 32-bit big-endian representation.

Host-to-provider control frames are:

```text
+----------------------+------------------------------+
| JSON header bytes u32| versioned JSON envelope ... |
+----------------------+------------------------------+
```

Provider-to-host frames are:

```text
+----------------------+----------------------+----------------+-------------+
| JSON header bytes u32| binary body bytes u32| JSON envelope  | binary body |
+----------------------+----------------------+----------------+-------------+
```

The JSON header is limited to 64 KiB. The binary body is limited to 16 KiB and
must be empty for every message except `audio_packet`. An `audio_packet` declares
`payload_bytes`; the declaration and actual body length must match and both must
be non-zero. Length limits are checked before allocating either section.

The binary body is separate from JSON rather than base64-encoded and never
enters Bridge's 64 KiB telemetry stream. A live provider will place one encoded
Opus packet in each audio-packet body. Slice 1's fixed fake bytes test framing
only and are not represented as a decodable recording.

Malformed JSON, invalid UTF-8, unsupported versions, zero or non-increasing
sequences, invalid messages, truncated frames, unexpected bodies, and oversized
lengths fail the provider session closed. A later supervisor owns timeouts and
bounded restart policy.

## Limits

| Contract value                |    Version-one limit |
| ----------------------------- | -------------------: |
| JSON control header           |               64 KiB |
| Encoded packet body           |               16 KiB |
| Sources per enumeration       |                   32 |
| Tracks per start request      |                    8 |
| Provider capabilities         |                    6 |
| Source channels               |                  1–8 |
| Declared native sample rate   |         8–384,000 Hz |
| Machine identifier            |       128 characters |
| Display label                 |       160 characters |
| State or event code           |        96 characters |
| Gap/dropout/backpressure span | 60 seconds at 48 kHz |

Source, track, and profile collections are bounded and must contain the required
unique identities. A valid enumeration may contain zero sources so provider and
hardware unavailability remain representable. JSON Schema captures the portable
structural limits; Rust validation additionally enforces cross-field rules such
as unique source and track identifiers, profile/channel compatibility, monotonic
clock ordering, and event-specific measurements.

## Source capabilities

Each source declares:

- a provider-owned machine identifier and a current display label;
- microphone, application output, endpoint output, simulator master mix,
  isolated COM1, isolated COM2, pilot radio, or copilot radio role;
- input or output direction;
- `isolated`, `mixed_output`, or `metadata_only` truth;
- separate availability and permission state;
- native channels and sample rate;
- supported versioned Opus profiles;
- hot-plug notification support; and
- operating-system, simulator, or external-application origin.

A metadata-only source has no audio profiles and cannot be captured. A source is
capture-ready only when it is non-metadata, available, permission is granted or
not required, and it declares at least one compatible profile. The host has a
separate explicit `request_permission` command so an operating-system prompt is
never an implicit effect of automatic capture. Application services will still
apply independent user consent before requesting permission or starting it.

Source labels and origin identifiers are untrusted and privacy-sensitive. They
remain excluded from plugins, optional-AI packets, Sentry, diagnostics, support
bundles, and public services. This protocol does not authorize their persistence.

## Opus profiles

Version 1 has a deliberately fixed catalogue:

| Profile               | Channels | Sample rate | Target bitrate |
| --------------------- | -------: | ----------: | -------------: |
| `pilot_microphone_v1` |        1 |      48 kHz |      48 kbit/s |
| `isolated_voice_v1`   |        1 |      48 kHz |      32 kbit/s |
| `mixed_stereo_v1`     |        2 |      48 kHz |     128 kbit/s |

The domain model provides overflow-safe encoded-size estimates from these
targets. Audio packets use the Opus 48 kHz timebase and accept only 2.5, 5, 10,
20, 40, or 60 millisecond durations: 120, 240, 480, 960, 1,920, or 2,880 sample
frames per channel.

## Lifecycle

The version-one control flow is:

1. The host sends `hello` for one expected provider identity.
2. The provider returns `hello`, then bounded `state` transitions.
3. The host requests `enumerate_sources`; the provider returns a revisioned
   capability list.
4. After a future explicit user action, the host may send `request_permission`
   for one exact source. The provider returns a revised capability list; capture
   start itself does not implicitly prompt.
5. The host may exchange `synchronize_clock` messages. The provider echoes the
   host send value and reports ordered provider receive/send monotonic values;
   the host will later record its receive value and calculate correlation and
   uncertainty.
6. After future application-owned consent, the host sends `start_capture` with
   one to eight exact source, track, and profile selections.
7. The provider emits `capture_started`, encoded `audio_packet` frames, bounded
   `level` observations, and explicit `capture_event` facts.
8. The host sends `stop_capture` or `shutdown`; the provider reports the exact
   stop reason and final state.

Capture events represent permission delay or denial, source loss/change, gaps,
dropouts, drift, backpressure, and encoder failure. Gap-like events declare a
bounded affected-frame count. Drift declares a bounded signed parts-per-million
observation. Events do not silently splice media, replace sources, or alter
simulator and operating-system volume.

## Schemas and fixtures

- `schemas/audio-provider-manifest-v1.schema.json`
- `schemas/audio-provider-package-manifest-v1.schema.json`
- `schemas/audio-source-capability-v1.schema.json`
- `schemas/audio-provider-envelope-v1.schema.json`
- sanitized hello, permission, sources, clock, packet-header, packet-body, and
  event fixtures under `schemas/fixtures/`

The fake-provider black-box tests prove deterministic handshake, enumeration,
explicit permission transition, clock exchange, microphone start, packet
transport, level and gap reporting, stop, shutdown, unavailable-source rejection
without fallback, and rejection of a non-increasing host sequence. They do not
prove native capture or valid Opus encoding.

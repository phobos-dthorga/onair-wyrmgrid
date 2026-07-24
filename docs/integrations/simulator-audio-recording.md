# Simulator-synchronised audio recording

Status: managed external capture-provider and codec-provider packages,
debug-only Windows microphone capture, and a first-party Opus codec package
implemented; native capture and codec sidecars are not released or
live-certified

WyrmGrid will offer optional audio recording aligned with a local simulator
telemetry session. Codec choice is an end-user selection implemented through a
specialised out-of-process codec-provider contract. WyrmGrid's own Opus
implementation uses that same contract. This document defines the product
truth, platform boundary, storage model, privacy rules, and validation required
before capture can be described as available.

Audio recording is not part of Hoardmind or any other AI workflow. Audio must
never enter an optional-AI packet.

## Product boundary

Audio is recorded **alongside** telemetry, not inside the current WyrmGrid
Bridge stream:

```text
Simulator API -> Bridge provider -> validated telemetry facts --------+
                                                                    |
Audio source -> Capture Provider v2 -> bounded PCM                    |
                                      -> selected Codec Provider v1   |
                                      -> encrypted encoded tracks ----+-> one session timeline
```

WyrmGrid Bridge protocol version 1 remains a bounded 64 KiB JSON control and
telemetry boundary. It is unsuitable for continuous PCM or encoded media. Audio
Capture Provider protocol version 2 supplies bounded PCM to the host; Audio
Codec Provider protocol version 1 converts it to bounded encoded packets.
Default-off consent, application orchestration, encrypted media lifecycle,
authenticated packet inspection/export, a deterministic fake capture provider,
managed external `.wyrmaudio` and `.wyrmcodec` lifecycles, a debug-only Windows
microphone provider, and a first-party Opus codec package now exist. The fake
provider and Opus codec have separately installable reference packages; the
native sidecars are not released or live-certified. Either process may fail or
be absent without taking down WyrmGrid, telemetry, or the simulator.

The Rust application service is authoritative for consent, source selection,
session lifecycle, time correlation, storage policy, deletion, and export.
Capture providers enumerate and capture approved sources. Codec providers
encode only host-selected PCM. Interface controls display state and delegate
actions; they do not decide recording policy.

## Implemented non-native slices

Slice 1 originally implemented the version-one capture contract. Slice 5
supersedes its encoded-packet path with [Audio Capture Provider version
2](audio-capture-provider-protocol.md), [Audio Codec Provider version
1](audio-codec-provider-protocol.md), codec-neutral stable profiles, separate
bounded PCM and encoded framing, sanitized fixtures, and deterministic process
tests. Version-one capture fixtures remain compatibility evidence.

Slices 2–5 implement independent persisted consent, explicit permission
requests, managed external-provider selection, schema-22 package state,
authenticated external packet segments, recovery, retention, tombstoned deletion, portable-
backup omission, bounded authenticated packet inspection, and separately
warned plaintext packet export. These slices do not enable a microphone or
native simulator capture and do not establish audible playback or live
certification.

Slice 5A implements a debug-only Windows microphone provider through CPAL/WASAPI
with explicit source selection, explicit permission probing, hashed raw device
identities, bounded non-blocking capture queues, PCM conversion, and
backpressure events. Slice 5B implements per-source codec selection, schema-20
codec provenance, host orchestration, and `dev.wyrmgrid.opus` as a normal
out-of-process codec provider. Slice 5C adds `.wyrmcodec` inspection,
installation, immutable updates, enable/disable, rollback, removal, first-party
seeding, and a packaged synthetic capture-to-Opus-to-encrypted-playback test.
Automated tests use synthetic audio and never open a real microphone.

## Simulator and operating-system support

X-Plane 12 confirms that the feature needs a genuinely cross-platform core even
though MSFS 2024 is Windows-specific.

| Simulator  | Desktop systems relevant to WyrmGrid | Telemetry path                        | Audio position                                                                                                                           |
| ---------- | ------------------------------------ | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| MSFS 2024  | Windows                              | SimConnect Bridge provider            | Microphone and explicitly selected Windows application, endpoint, or simulator mix; documented COM state is metadata, not isolated audio |
| X-Plane 12 | Windows, macOS, Ubuntu LTS           | Planned local Web API Bridge provider | Platform capture plus a feasibility spike for named X-Plane audio groups                                                                 |

X-Plane officially supports Windows 10 or 11 64-bit, macOS 12 or later, and
Ubuntu LTS, with Ubuntu the Linux family it tests. Its plugin SDK publishes
Windows, Linux, Intel Mac, and Apple Silicon support. Other Linux distributions
must not be advertised merely because a build happens to work.

### MSFS 2024

The documented indexed COM variables include availability, active and standby
frequency, receive, transmit, status, and volume. They can create attributed
timeline facts such as "COM1 transmit selected" or "COM2 receive enabled".
They do not establish access to PCM from an individual radio.

Initial Windows capture candidates are:

- the explicitly approved pilot microphone;
- the MSFS process output where process capture is proven;
- an explicitly selected output endpoint, including a dedicated simulator or
  headset endpoint; and
- an explicitly selected external ATC application.

A mixed MSFS or endpoint track is labelled **mixed output**. Radio state may be
displayed beside it, but WyrmGrid must not infer which audible sample came from
COM1, COM2, built-in ATC, an add-on, or another application.

### X-Plane 12

X-Plane exposes output channel groups for incoming COM1 and COM2 speech, pilot,
copilot, exterior aircraft, exterior environment, interior, UI, ground, and
master audio. The SDK can return a reference to a selected output group. This
makes isolated capture plausible, not proven.

The first feasibility spike must determine whether a thin plugin can attach a
read-only, non-blocking tap without changing routing, volume, latency, aircraft
sound, simulator stability, or third-party-aircraft behaviour. It must also
resolve FMOD/API licensing, dedicated-headset routing, bank or device reloads,
plugin signing and installation, Apple Silicon packaging, Linux compatibility,
local transport authentication, and clean removal.

If the spike succeeds, the plugin does only three things: identify a negotiated
named group, timestamp bounded sample frames, and deliver them without blocking
X-Plane. It does not encode files, select retention, read WyrmGrid data, or make
recording decisions. Buffer pressure drops audio and reports a gap rather than
stalling the simulator.

## Source capabilities and truthful labels

An audio provider advertises sources at runtime. A stable application model
uses these source classes:

- `microphone_input`;
- `application_output`;
- `output_endpoint`;
- `simulator_master_mix`;
- `isolated_com1`;
- `isolated_com2`;
- `pilot_radio`; and
- `copilot_radio`.

Each source also declares:

- supported platform and provider;
- availability and permission state;
- stable provider-owned source identifier and current display label;
- input or output direction;
- channel count, native sample rate, and accepted Opus profile;
- `isolated`, `mixed_output`, or `metadata_only` truth class;
- whether hot-plug and device-change notifications are supported; and
- simulator or external-application provenance when it is known.

The interface lists only negotiated sources. It uses labels such as **X-Plane
COM1 — isolated**, **MSFS output — mixed**, or **COM2 — telemetry markers
only**. A source does not become isolated because its provider can observe a
matching frequency or receive flag.

External voice clients such as online-network or AI ATC applications are
separate processes. WyrmGrid records one only after the user selects that named
application or its dedicated endpoint. It does not capture the entire desktop
as a shortcut and does not infer that unrelated process audio belongs to the
flight.

## Recording controls

Audio permission and automation remain separate from telemetry recording:

| Choice                                                | Default | Effect                                                                   |
| ----------------------------------------------------- | ------- | ------------------------------------------------------------------------ |
| Enable audio recording                                | Off     | Allows the user to review and select sources                             |
| Record audio with manually started telemetry sessions | Off     | Starts only the previously approved sources                              |
| Record audio with automatically started flights       | Off     | Depends on telemetry automation but requires its own persistent approval |
| Include full simulator mix                            | Off     | Adds ambience and simulator sound where supported                        |
| Include a named external application                  | Off     | Records only the explicitly selected application or endpoint             |

An active microphone or communications recording has a persistent accessible
indicator in the main window and any future tray or in-simulator controller.
The indication must not rely on colour, animation, or sound alone. Starting a
provider, accepting legal documents, granting a plugin capability, or enabling
telemetry automation cannot satisfy audio consent.

Each available source presents:

- record on/off;
- a bounded live level meter and clipping indication;
- source availability, permission, and isolation state;
- playback/export mute and solo; and
- non-destructive playback/export volume.

Recording trim is distinct from playback volume. The normal recording path
uses unity gain unless the user deliberately changes a supported input trim.
WyrmGrid never silently writes the simulator's COM volume or the operating
system's device volume. A simulator-reported COM volume remains a read-only
fact.

## Timeline and synchronization

The audio session links to one `SimulatorSession` but retains its own lifecycle.
The host establishes a monotonic session origin. Every track records its start
offset, sample rate, first sample-frame index, segment frame range, and provider
sequence. UTC is display metadata and never the sole synchronisation clock.
The schema keeps the host monotonic origin nullable: the current fake-provider
orchestration leaves it unavailable because it has not yet performed and stored
the version-one clock-correlation exchange. It does not substitute zero or UTC
for unavailable monotonic evidence.

The following remain explicit:

- permission delays and tracks that start after telemetry;
- simulator pause or non-unit simulation rate;
- telemetry disconnect while audio continues;
- audio-source loss while telemetry and other tracks continue;
- provider restart, device hot-plug, or source identity change;
- sample-clock drift and bounded resynchronisation observations;
- dropped buffers, encoder backpressure, and disk-write failure; and
- aircraft or simulator identity changes that end the associated session.

Pausing the simulator does not compress the audio timeline. A pause becomes a
telemetry event while audio continues unless the user stops it. A transient
telemetry gap likewise does not discard speech. WyrmGrid never joins either
side of a discontinuity as though it were continuous.

A disappearing device does not silently fall back to the operating-system
default, which could capture an unintended microphone. The track becomes
unavailable and records a source-loss event until the approved source returns
or the user selects another one.

## Codec providers and first-party Opus

Opus is open, royalty-free, designed for speech and music, and suitable for
storage as well as interactive audio. Its reference implementation uses the
three-clause BSD licence. These properties make it WyrmGrid's first-party and
initial default codec, but not the only codec end users may eventually install.

| Codec           | Suitability | Decision                                                                 |
| --------------- | ----------: | ------------------------------------------------------------------------ |
| **Opus**        |  **9.5/10** | First-party provider for voice, radio, and simulator mix                 |
| FLAC            |      7.5/10 | Possible explicit lossless export; too large for routine working storage |
| Vorbis          |        6/10 | Open and usable, but offers no compelling advantage over Opus here       |
| PCM in WAV/RF64 |        4/10 | Diagnostic interchange only; uncompressed storage is excessive           |
| Speex           |        2/10 | Rejected for new work because Opus supersedes it                         |

Stable profile roles are deliberately few and versioned. Codec manifests bind
those roles to their own format, media type, bitrate, channels, and packet
duration. The first-party Opus provider declares:

| Track role                   | Channels | Sample rate | Target bitrate |
| ---------------------------- | -------: | ----------: | -------------: |
| Pilot microphone             |     Mono |      48 kHz |   40–48 kbit/s |
| Isolated COM or voice        |     Mono |      48 kHz |   24–32 kbit/s |
| Simulator or application mix |   Stereo |      48 kHz |  96–128 kbit/s |

At the upper targets, a microphone uses about 21.6 MB per hour, one isolated
radio uses about 14.4 MB per hour, and a stereo simulator mix uses about
57.6 MB per hour before small container and encryption overheads. Microphone,
COM1, COM2, and simulator mix together are approximately 108 MB per hour. The
interface must show an estimated size from the actual selected profiles before
automatic capture is enabled.

Local recording does not need Opus in-band forward-error correction. Silence
and timing remain represented rather than using discontinuous transmission to
collapse the session. Speech and mixed-audio encoder modes may differ behind
the same versioned profile catalogue.

## External media and SQLite metadata

Working media uses independently recoverable authenticated WyrmGrid encoded-
packet segments. The initial envelope is deliberately not described as Ogg,
Matroska, or another standard container. Codec identity and media type are
recorded separately. Live tests may justify a different bounded segment
duration. Segmentation limits crash damage, permits per-track gaps, and avoids
rewriting a multi-hour file.

Audio bytes never become a SQLite BLOB. Append-only migration 18 adds
application-owned records equivalent to:

- `AudioRecordingPreferences`: separate consent and automation choices,
  retention, and storage budget;
- `AudioRecordingSession`: linked simulator session, origin, start/end, status,
  and aggregate size;
- `AudioTrack`: role, truth class, capture-provider/source provenance, codec-
  provider identity/version, codec ID/media type, profile, channels, sample
  rate, offsets, and status;
- `AudioTrackSegment`: opaque storage key, frame range, duration, byte size,
  integrity hash, encryption version, and finalisation state; and
- `AudioCaptureEvent`: permission, source, gap, dropout, drift, provider,
  storage, and user-marker events.

Absolute local paths, PCM, encoded packets, voices, and device or application
labels are not copied into the database. Display labels may remain session-only
unless a reviewed usability need justifies bounded encrypted persistence.

Append-only migration 20 adds the selected codec-provider identity to source
selections and snapshots provider identity/version plus codec ID/media type on
tracks. Schema-19 rows receive `dev.wyrmgrid.opus`, `legacy-unversioned`,
`opus`, and `audio/opus`, the only previously implied format without an
invented historical provider version, without rewriting migration 18.

Append-only migration 22 preserves every schema-21 managed plugin, simulator
provider, and audio provider version and active/rollback state while extending
the constrained package-kind catalogue with `audio_codec_provider`.

The version-one media envelope uses XChaCha20-Poly1305 with a fresh random
24-byte nonce per segment. HKDF-SHA256 derives a purpose-separated media key
from the installation's uniformly random database key using fixed versioned
salt and info labels. Authenticated data binds the envelope header, opaque
storage key, session, track, segment index, first frame, and frame count. A
stored SHA-256 envelope digest provides early corruption evidence; successful
AEAD authentication remains authoritative.

The writer creates a pending segment, writes and authenticates bounded content,
finalises it atomically, and then marks the metadata complete. Startup detects
unfinished segments and orphaned files without inventing duration or
continuity. Disk-full or encryption failure stops affected audio tracks and
leaves telemetry recording operational.

## Retention, deletion, backup, and export

Audio follows the linked session's deletion and pinning choices but also has a
user-visible total storage budget. Automatic pruning never removes media from
an active session. Deleting a completed simulator recording schedules its audio
segments and metadata together. A failed file deletion is tombstoned, hidden
from normal use, retried, and reported without claiming secure erasure from
SSDs, filesystem snapshots, cloud-synchronised folders, or backups.

Default portable backups remain bounded and therefore exclude audio media.
Backup creation must say that audio is omitted, and a restored session must
display **audio not included in backup** rather than a generic corruption
error. An optional media-inclusive backup is deferred until its size estimate,
streaming encryption, cancellation, restore, and cross-platform behaviour are
designed and tested.

Normal JSON and CSV telemetry exports do not include audio or audio paths. An
audio export is a separate deliberate action with source selection and a clear
plaintext warning. Initial export candidates are individual Ogg Opus tracks or
a named multi-track Matroska Audio file; FLAC may be offered only as an explicit
lossless export. Exported copies are outside WyrmGrid's retention and deletion
control.

## Privacy and security requirements

Recorded voices can identify the pilot or other people and may contain
callsigns, account details, conversations, alerts, or unrelated background
speech. Device and application names may also reveal usernames or installed
software. Before implementation ships:

- the Privacy Notice and data inventory must add the exact audio categories,
  purpose, location, retention, deletion, backup, and export behaviour;
- recording-law and consent obligations must receive professional review for
  intended distribution jurisdictions;
- VATSIM, IVAO, SayIntentions, and any other captured service's current rules
  must be reviewed rather than assuming local recording is permitted;
- Windows, macOS, and Linux permission prompts and indicators must be tested on
  every packaged target;
- source identifiers, labels, and failures must be redacted from diagnostics;
- general plugins receive no audio capability; an explicitly selected codec
  provider necessarily receives only that selected source's transient PCM and
  requires the separate native-code package trust disclosure; and
- support tooling, Sentry, optional AI, crash attachments, and public services
  must be unable to receive audio or media paths by default.

The in-process X-Plane feasibility plugin is first-party only. It must not
create a general in-process community ABI or bypass provider signing, package
integrity, install-root, update, rollback, and removal requirements.

## Failure and unavailable behaviour

- No audio provider: telemetry and WyrmGrid remain fully usable.
- Permission denied: the source stays off and explains how to recover; no retry
  loop repeatedly prompts the user.
- Source missing: other approved tracks continue and the missing track records
  an explicit gap.
- Encoder or writer failure: affected tracks stop safely; telemetry continues.
- Provider crash: WyrmGrid reports interruption and applies bounded restart
  policy rather than silently changing source.
- X-Plane tap backpressure: samples drop and become a measured gap; X-Plane is
  never blocked.
- Clock drift: WyrmGrid records anchors and exposes uncertainty; it does not
  silently time-stretch evidence during capture.
- Application crash: completed segments remain recoverable and the active
  segment is validated or marked interrupted on restart.

## Validation plan

Application and protocol tests must be hardware-independent wherever possible.
A deterministic fake audio provider supplies exact sample clocks, source
changes, gaps, permission delays, dropouts, drift, and failures on every
supported build platform.

Required automated coverage includes:

- provider handshake, version, identity, capability, frame, rate, timeout, and
  shutdown validation;
- separate audio and telemetry consent, including every automatic-start
  combination;
- source enumeration, truthful isolation labels, hot-plug, and no silent
  fallback;
- monotonic alignment, pause, simulator-rate change, telemetry gaps, audio
  gaps, drift, and provider restart;
- Opus profile, duration, channel, bitrate, segment, corruption, and decoding
  fixtures;
- bounded memory, backpressure, disk-full, interrupted write, orphan cleanup,
  retention, pinning, quota, and deletion cases;
- encryption envelope, wrong/missing key, tamper rejection, key-version, and
  recovery fixtures;
- backup omission and restored **audio unavailable** presentation;
- plugin, Sentry, diagnostics, support, and optional-AI exclusion tests; and
- accessible recording state, keyboard operation, non-colour indication, and
  narrow-window presentation.

Outside-repository live certification must cover representative microphones,
headsets, output routing, simulator versions, default and third-party aircraft,
long flights, pause, device changes, simulator restart, and application crash.
X-Plane certification repeats on Windows, Intel and Apple Silicon macOS where
supported, and supported Ubuntu LTS. Passing one operating system or aircraft
does not establish another.

## Delivery sequence

1. Define the original Audio Capture Provider protocol, fake-provider fixtures,
   application models, and profile catalogue. (Implemented; the encoded-media
   portion is archived as version 1 compatibility evidence.)
2. Add independently default-off master, manual, automatic, and source-specific
   consent plus explicit permission and fake-provider orchestration. (Implemented
   without a native provider.)
3. Add append-only metadata, independently authenticated external segments,
   startup recovery, retention, pinned-session protection, tombstoned deletion,
   and portable-backup omission. (Implemented in schema 18.)
4. Add bounded authenticated packet inspection and separately warned plaintext
   track export without claiming decoding or a standard media container.
   (Implemented.)
5. Separate capture and codecs: implement Audio Capture Provider version 2,
   Audio Codec Provider version 1, user codec selection, schema-20 provenance,
   a debug-only Windows microphone provider (5A), and first-party Opus as a
   normal codec provider (5B), followed by managed `.wyrmcodec` packaging and
   synthetic packaged-chain grounding (5C). (Implemented without live-device
   certification.)
6. Extend the Windows provider to explicitly selected MSFS, application, or
   endpoint output, with SimConnect COM facts presented as metadata only.
7. Complete the X-Plane local Web API telemetry provider across WyrmGrid's
   supported Windows, macOS, and Linux targets.
8. Deliver approved microphone and mixed-output capture for those X-Plane
   targets where platform certification passes.
9. Run the X-Plane named-audio-group feasibility spike. Add isolated COM1,
   COM2, pilot, or copilot sources only after the separate review succeeds.
10. Add explicitly selected external ATC application capture where each platform
    can enforce truthful source selection and current service rules permit it.
11. Add verified codec-provider discovery, exact package identity and integrity,
    updates, rollback, removal, and explicit trust presentation before
    accepting user-installed codec executables. (Implemented for deliberate
    unsigned local packages; publisher signing, authenticated updates,
    revocation, and OS resource limits remain release hardening.)
12. Update the Privacy Notice, legal versions, threat model, user guide, licence
    bundle, installers, and release notes only for capabilities actually ready to
    ship.

No stage changes the application semantic version or claims live simulator
support without maintainer authorization and the required release evidence.

## Decisions deliberately deferred

- packaged Windows audio dependencies and minimum operating-system version;
- community codec publisher signing, authenticated updates, revocation, and OS
  resource controls;
- Windows application, endpoint, and process-loopback capture;
- whether X-Plane FMOD tapping is stable and distributable;
- native-provider segment duration after benchmarks and live certification;
- media-inclusive portable backup;
- initial playback editing, waveform, transcription, or voice analysis; and
- capture of multiplayer or online-network voices beyond an explicitly selected
  and policy-reviewed source.

Transcription, speech recognition, voice identification, hosted upload,
automatic public sharing, and AI analysis are not part of the approved feature.

## Official references

- [X-Plane 12 system requirements](https://www.x-plane.com/kb/x-plane-12-system-requirements/)
- [X-Plane plugin SDK downloads and platforms](https://developer.x-plane.com/sdk/plugin-sdk-downloads/)
- [X-Plane audio-bus enumeration](https://developer.x-plane.com/sdk/XPLMAudioBus/)
- [X-Plane output channel-group access](https://developer.x-plane.com/sdk/XPLMGetFMODChannelGroup/)
- [MSFS 2024 aircraft radio-navigation variables](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimVars/Aircraft_SimVars/Aircraft_RadioNavigation_Variables.htm)
- [Opus overview](https://opus-codec.org/)
- [Opus licence and royalty-free grants](https://opus-codec.org/license/)
- [FLAC licence](https://xiph.org/flac/license.html)
- [Vorbis overview and licence position](https://xiph.org/vorbis/)
- [Speex status](https://www.speex.org/)

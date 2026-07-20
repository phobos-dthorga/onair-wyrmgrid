# ADR-0017: Simulator-synchronised audio recording through capability providers

Status: accepted

## Context

WyrmGrid already records bounded, translated simulator telemetry for later
debriefing. Pilots may also want an aligned record of their microphone,
simulator output, radio traffic, or an explicitly selected ATC application.
Audio has different volume, timing, privacy, permission, and failure properties
from the one-hertz JSON facts carried by WyrmGrid Bridge.

Simulator support is not uniform. MSFS 2024 is a Windows desktop target whose
documented SimConnect surface exposes radio state but no reviewed isolated COM
audio stream. X-Plane 12 runs on Windows, macOS, and supported Ubuntu LTS
systems. Its plugin SDK exposes named COM1, COM2, pilot, copilot, and master
output groups, but safe recording from those groups has not yet been proven.

Storing encoded media inside SQLite would enlarge and churn the encrypted
database, portable backups, transactions, and recovery path. Reusing telemetry
recording consent for voices would also collapse materially different user
choices.

## Decision

WyrmGrid will treat simulator-synchronised audio as an optional core recording
capability with these fixed boundaries:

- **Opus is the implementation codec.** Segmented Ogg Opus is the preferred
  working representation. A future explicit export may offer another container
  or a lossless format without changing the stored default.
- **SQLite stores metadata only.** Encoded audio remains in encrypted, bounded
  local media segments addressed by opaque storage identifiers. A future
  append-only migration may add session, track, segment, event, retention, and
  integrity metadata; no released migration is edited.
- **Audio media does not use WyrmGrid Bridge protocol version 1.** Bridge remains
  the source of translated simulator and radio facts. A separately versioned,
  supervised Audio Capture Provider boundary will negotiate sources and control
  capture. Its bounded media transport will be decided and tested separately
  from Bridge's 64 KiB JSON channel.
- **The application owns policy.** Rust application services own consent,
  session identity, monotonic correlation, retention, deletion, export, and
  presentation. Native providers capture and encode; Svelte remains
  presentational.
- **Capabilities describe truth.** Sources declare their role and whether their
  samples are isolated, a mixed output, or unavailable with metadata only.
  COM1 or COM2 is never presented as an audio track merely because radio state
  is observable.
- **Audio consent is independent and default-off.** Legal acknowledgement,
  provider launch, telemetry recording, automatic telemetry recording, and
  plugin grants do not enable a microphone or communications source. Full
  desktop audio is never selected implicitly.
- **Audio stays private by default.** Audio content, device labels, application
  identities, and communications are excluded from plugins, Sentry,
  diagnostics, optional AI handoffs, public services, and support bundles.
  Plaintext export is a separate deliberate disclosure.
- **Provider failure degrades locally.** Audio loss does not stop telemetry,
  and telemetry loss does not silently splice or relabel audio. Source changes,
  drift, gaps, dropouts, delayed permission, and interruption remain explicit.

MSFS 2024 initially permits Windows microphone, explicitly selected
application or endpoint output, and simulator-mix capture where the Windows
provider proves it. Indexed COM facts remain timeline metadata unless a future
supported API supplies isolated samples.

X-Plane 12 may use its local Web API provider for telemetry on Windows, macOS,
and supported Linux systems. A future first-party in-process audio tap is
permitted only after a focused stability, licensing, signing,
installation/removal, authentication, backpressure, and live-capture decision.
If approved, that tap is a minimal sample source with no WyrmGrid business
logic; supervision, encoding, storage, and policy remain outside X-Plane.

## Consequences

One application model can support MSFS and X-Plane without pretending their
audio surfaces are equivalent. Opus keeps multi-hour, multi-track recordings
bounded while preserving useful speech and simulator sound. Segmented external
storage improves crash recovery and keeps ordinary database operations and
default portable backups bounded.

The feature requires a new provider contract, encrypted-media format, storage
quota and backup policy, platform capture implementations, append-only
migration, interface, tests, Privacy Notice revision, legal review, and
outside-repository live certification before it can ship. X-Plane per-bus
capture remains a candidate rather than an availability claim. The current
application, Bridge protocol, telemetry recording, plugin permissions, and
Privacy Notice remain unchanged by this decision.

## Implementation note — 2026-07-20

The independently versioned Audio Capture Provider protocol version-one
foundation is implemented with stable source and Opus-profile models, bounded
JSON control headers, separately bounded encoded-packet bodies, schemas,
sanitized fixtures, and a deterministic development-only fake provider. This
resolves the deferred provider media-transport shape without changing Bridge
version 1. It does not implement consent, storage, native capture, packaging, or
user-facing availability; all remaining decision boundaries above stay in force.

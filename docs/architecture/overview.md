# Architecture overview

```text
External providers    Simulator sidecars    Audio sidecars (planned)
        |                      |                       |
        v                      v                       v
 Rust provider adapters  WyrmGrid Bridge    Audio capture boundary
        |                      |                       |
        +--------------------> application <----------+
                                  |
                        +---------+---------+
                        |                   |
               SQLCipher SQLite         plugin broker
                   (Hoard)                   |
                        |              external plugins
                        v
                    Tauri commands
                        |
                        v
            Svelte presentation <--- Fluent catalogue + data-only language pack
                        |          |          |
                    MapLibre   Three.js   WyrmChart
                     (Atlas)   (weather)  (ECharts)
```

The dependency direction points inward. Interface and infrastructure adapters
depend on application-owned domain contracts; domain code does not depend on
Tauri, SQLite, HTTP, MapLibre, Three.js, or a plugin language.

## Proposed hosted ecosystem boundary

WyrmGrid remains usable without project-operated infrastructure. A future
SvelteKit website may present documentation and catalogue views, while a
separate Rust Aerie service owns package, publisher, compatibility, moderation,
revocation, and authorization rules. Quarantined validation workers never
execute uploaded code, and signed immutable public targets remain distinct from
private user storage.

If an optional private vault proceeds, its first scope is opaque storage of an
existing client-encrypted `.wyrmbackup`; the client retains the password and
plaintext. It uses separate service credentials, database roles, storage roots,
retention, audit, and incident boundaries from the public catalogue. It does
not imply record-level synchronization. Website, catalogue, identity, vault,
DNS, or signing-service unavailability must not block ordinary local use.

See
[ADR-0019](decisions/0019-hosted-web-aerie-and-private-vault.md), the
[implementation plan](../operations/hosted-platform.md), and the
[licensing and compliance register](../legal/hosted-platform-licensing.md).

## Development-assistance boundary

AI assistants are outside the WyrmGrid product architecture. WyrmGrid has no AI
runtime, build, test, contribution, plugin-protocol, CI, or release-publication
dependency. Hoardmind is the maintainer's private local assistant; it is not a
WyrmGrid module, service, bundled sidecar, supported integration, or source of
project authority.

Development and release work can be performed entirely by a person. A
contributor may instead choose the repository's versioned, profile-driven local
helper for bounded change-impact, test-matrix, documentation-sync, synthetic
fixture, bounded implementation-patch, sanitized failure-triage, or release-
curation drafts. Its adapters support user-selected Ollama models and
unauthenticated local servers that implement the OpenAI-compatible model-list
and chat-completion endpoints, all restricted to that machine's loopback
interface. Drafts remain untrusted input, but independent Codex semantic review
is reserved for high-benefit or critical work. Valid lower-benefit output still
passes deterministic gates without spending frontier-model review resources.
Drafts are never autonomously chained, and profiles and temporary artifacts
remain outside the application. LAN, authenticated, or hosted AI providers are
intentionally not interchangeable with this local boundary; supporting one
would require a separate privacy, authentication, data-flow, and threat-model
decision.

Generated-contribution attribution is a separate maintainer-side control plane.
A local broker may exchange an App JWT for a short-lived, repository-scoped
installation token after validating a human-approved patch and manifest hash.
The assistant has no access to that broker credential. The App creates only one
commit and an identity-bound branch. After the token is discarded, the human
maintainer's authenticated GitHub CLI creates the draft PR. The App is not a
WyrmGrid plugin, runtime sidecar, build input, CI model call, or landing
authority.

## Localization boundary

Domain models remain language-neutral. Application services return semantic
states, stable message or error codes, and formatting arguments; they do not
select a locale. Svelte resolves presentation messages from the selected
community pack and falls back to the canonical `en-AU` catalogue. Raw provider
facts, user content, logs, and protocol identifiers are never treated as
translation keys. See [ADR-0010](decisions/0010-community-localization.md) and
the [language-pack authoring guide](../localization/README.md).

External providers include OnAir, SimBrief, SayIntentions.AI, aviation weather,
online networks, and optional navigation data. Each owns a private raw schema
and translates into application-owned operational snapshots before another
module consumes it.

## Data categories

Every user-facing value should be traceable to one of four categories:

1. OnAir fact;
2. external fact, such as simulator telemetry or weather;
3. calculated value;
4. recommendation.

Provenance records the source and observation time. Recommendations should also
explain their contributing factors rather than presenting an opaque score.

## Operational snapshot boundary

The application reconciles immutable flight-plan, route, weather, network, and
simulator-session snapshots. Similar fields from different providers remain
separate until a named application rule explains how they compare. Source,
generation or validity time, retrieval time, freshness, provider revision, and
AIRAC are retained where relevant.

The intended complete loop is:

```text
OnAir plan inputs + SimBrief OFP + weather
                    |
                    v
             WyrmGrid Dispatch
                    |
                    v
          simulator-neutral route ----------> MSFS 2024 Bridge
                    |                                  |
                    |                                  v
                    |                           simulator actuals
                    |                                  |
                    +----------------+-----------------+
                                     |
SayIntentions context ---------------+
                                     |
                                     v
                         planned-versus-actual analysis
```

See [ADR-0008](decisions/0008-provider-adapters-and-operational-snapshots.md)
and [ADR-0011](decisions/0011-core-simulator-capability-provider-sidecars.md),
plus the [external integrations programme](../integrations/README.md).

## Simulator-audio foundation and planned capture

Simulator-synchronised audio is adjacent to Bridge telemetry rather than part
of Bridge protocol version 1. The application owns separate default-off audio
consent, session correlation, retention, deletion, and presentation. A
separately supervised Audio Capture Provider v2 supplies capability-labelled
PCM to a separately supervised, user-selected Audio Codec Provider v1; SQLite
stores metadata while WyrmGrid encrypts bounded encoded segments. Audio, device
labels, and communications are unavailable to ordinary plugins and
observability.

The independently versioned capture and codec protocols, bounded control and
binary framing, source/profile domain models, schemas, fixtures, deterministic
fake capture, application consent and storage, debug-only Windows microphone
provider, codec-selection interface, and first-party Opus codec provider are
implemented. Packaging and live support are not.

MSFS 2024 capture is Windows-specific. X-Plane 12 provides the cross-platform
Windows, macOS, and supported Linux target, while its named COM audio groups
remain a feasibility candidate until a thin non-blocking tap is proven. See
[ADR-0017](decisions/0017-simulator-synchronised-audio-recording.md),
[ADR-0020](decisions/0020-out-of-process-audio-codec-providers.md), and the
[audio-recording plan](../integrations/simulator-audio-recording.md), plus the
[capture](../integrations/audio-capture-provider-protocol.md) and
[codec](../integrations/audio-codec-provider-protocol.md) protocol references.

## Extension boundary

Community plugins never link into the desktop process. The host launches a
declared entry point, grants approved capabilities, validates messages, applies
timeouts and size limits, and owns all privileged actions. Declarative map,
table, form, chart, notification, command, and inspector contributions come
before unrestricted custom UI.

## Maintainability boundary

WyrmGrid is designed to remain sustainable for one maintainer. Community-ready
boundaries must not require community-scale infrastructure. Each replaceable
technology has one application-owned adapter: OnAir JSON, SQLite, Tauri,
MapLibre, Three.js, and ECharts remain outside domain rules. The Three.js
weather renderer receives only a bounded host-owned presentation scene and
cannot reinterpret provider payloads or operational state. Its procedural 3D
density field changes local presentation texture only; it cannot manufacture a
weather cell or extend sourced coverage. Map projection round trips may fade a
decorative anchor behind the globe or horizon, but do not expose MapLibre's
renderer or imply shared terrain depth. See
[ADR-0018](decisions/0018-threejs-webgpu-weather-composition.md).

New abstraction is justified by a current use case, not the possibility of a
future contributor. See
[ADR-0004](decisions/0004-declarative-charts-and-complexity-budget.md).
Repeated presentation behavior follows the same rule: shared exploration,
date/time, authorization-label, and responsive-surface primitives are reused,
while domain-specific field meaning remains local. See the
[presentation and exploration audit](reusable-presentation-and-exploration.md).

The presentation runs in Tauri's platform webview: Chromium-based WebView2 on
Windows and WebKit-backed views on macOS and Linux. WRY is the application
boundary, while cross-engine behaviour is verified rather than replaced by a
bundled browser runtime. See [ADR-0005](decisions/0005-system-webviews.md).

## Observability boundary

Domain and application types do not depend on Sentry. They expose typed,
sanitized diagnostic outcomes that optional Rust and SvelteKit adapters can send
at the desktop composition boundary. Losing the telemetry service must never
block startup, storage, API access, calculations, or plugin operation.

Community plugins do not receive host DSNs, upload credentials, or a transparent
path for submitting arbitrary events through the host. The collection policy,
hosting decision, and rollout gates are documented in
[ADR-0007](decisions/0007-hosted-sentry-observability.md) and the
[observability plan](../operations/observability.md).

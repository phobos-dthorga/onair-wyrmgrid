# Architecture overview

```text
External providers       Simulator sidecars
    |                           |
    v                           v
Rust provider adapters   WyrmGrid Bridge
    |                           |
    +--------> application <----+
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
     |          |
 MapLibre    WyrmChart
  (Atlas)    (ECharts)
```

The dependency direction points inward. Interface and infrastructure adapters
depend on application-owned domain contracts; domain code does not depend on
Tauri, SQLite, HTTP, MapLibre, or a plugin language.

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
interface. Drafts remain untrusted input for normal review, are never
autonomously chained, and profiles and temporary artifacts remain outside the
application. LAN, authenticated, or hosted AI providers are intentionally not
interchangeable with this local boundary; supporting one would require a
separate privacy, authentication, data-flow, and threat-model decision.

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
MapLibre, and ECharts remain outside domain rules.

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

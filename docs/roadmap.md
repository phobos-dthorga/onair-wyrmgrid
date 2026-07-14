# Roadmap

## Foundation

- Repository, governance, security, CI, and release automation
- Tauri/Svelte/MapLibre application shell
- Stable domain provenance and plugin manifest v1 groundwork
- Read-only credential-safe OnAir adapter and SQLite migration ownership
- Versioned first-run Terms, Privacy Notice, persistent user control, and
  opt-in privacy-filtered Rust and Svelte error monitoring (implemented);
  public Sentry operational review and native symbol upload remain
- Semantic styling, four built-in appearances, and a persisted, contrast-checked,
  data-only community theme manifest (implemented); authoring, export, deletion,
  and any curated distribution remain later ecosystem work
- Canonical `en-AU` Fluent catalogue, persisted language preference, bounded
  data-only community language packs, per-message English fallback, and initial
  shell/Theme/Dispatch migration (implemented); complete string extraction,
  pseudo-locales, reviewed translations, RTL certification, export, and deletion
  remain localization work

## Observability foundation

- Record the hosted Sentry, privacy, and adapter boundaries in ADR-0007
- Introduce a typed cross-boundary error contract and explicit degraded outcomes
- Add error-only Rust and SvelteKit adapters for deliberate maintainer builds
- Correlate releases, source maps, and native debug information in CI
- Add user-visible disclosure and a telemetry control before public transmission
- Defer performance tracing, logs, replay, profiling, and native minidumps until
  measured needs justify their privacy and operational cost

The implementation sequence and operating thresholds live in the
[observability plan](operations/observability.md).

## Vertical slice 1: company and fleet

- Session-only connection probe with sanitized diagnostics (implemented)
- Optional operating-system credential store
- Company connection plus initial fleet, aircraft, airport, and FBO translation
  (implemented)
- Timestamped company-data refresh, compacted SQLite history, restart-time offline
  fallback, and explicit live/cached/offline state (implemented)
- Hoard Timeline fleet history: persistent LIVE/HISTORICAL mode indicator,
  as-of selection, return-to-present control, and fleet growth/composition
  charts, plus FBO-network growth from independently timestamped observations
  (implemented)
- Conservatively rate-protected manual and user-configured automatic company
  synchronization (implemented)
- Atlas fleet and FBO markers, independent layer toggles, shared map fitting,
  and linked inspectors (implemented)
- Bounded pending-job observations, Hoard-backed live/cached/offline state, and
  a read-only Jobs workspace (developer preview implemented; authenticated live
  contract confirmation remains)

## Vertical slice 2: external plugin proof

- Process supervisor, bounded framed messages, startup handshake, monotonic
  sequences, and graceful/forced shutdown (implemented)
- Forge permission review and append-only persisted deny-by-default grants
  (implemented)
- Core authorization service with revision-bound grants, local decision audit,
  and distinct legal/preference/grant/confirmation semantics (implemented)
- Sanitized fleet-read and host-rendered map-layer publication capabilities
  (implemented)
- Zero-dependency Python SDK and honest Fleet Locations example plugin
  (implemented)
- Developer-preview hardening: signed packages, publisher identity, OS sandbox,
  resource and message-rate ceilings, restart throttling, and conformance tests

## External operations track

This track begins after the fleet slice has SQLite persistence and safe offline
fallback. Its architecture may be prepared in parallel, but provider features do
not replace completion of the current vertical slice.

### Operational plan foundation

- Canonical provenance-aware `FlightPlanSnapshot` version 1 and route model from
  ADR-0008, with JSON Schema and sanitized fixtures (implemented)
- Stable SimBrief provider port with bounded private raw response translation
  (implemented)
- AIRAC, identifier, unit, timestamp, freshness, and reconciliation policies
  (initial import and fleet-comparison policies implemented; persistence remains)
- Host-owned import/export validation and deny-by-default plugin capabilities

### SimBrief and weather

- Latest SimBrief OFP import by an explicitly supplied Pilot ID or username,
  session-only Dispatch interface, and explicit clear action (developer preview
  implemented; authenticated compatibility certification remains)
- OnAir aircraft identity, exact model-label, current-airport, selected-job
  route, reported cargo, and expiry comparison with unavailable evidence exposed
  instead of inferred (implemented for the current read-only contracts)
- Explicitly requested, ten-minute session-cached AviationWeather.gov METAR and
  TAF airport context with raw coded text and provenance (implemented)
- Route advisories and explainable weather findings (airport observation context
  implemented; route hazard products remain)
- Dispatch-to-Atlas route projection with full-route framing and linked
  airports/fixes, followed by clickable SID/STAR geometry only after navigation
  and AIRAC resolution are implemented
- Live and Hoard-historical Atlas weather with a default Compatibility renderer,
  opt-in GPU-enhanced effects, and identical facts across rendering profiles
- SimBrief generation only after Navigraph approves the desktop flow and any
  required hosted-secret boundary receives a separate decision

### SayIntentions.AI

- Explicit opt-in read of the credential-bearing local `flight.json`
- Session-only SAPI key handling, followed by operating-system credential-store
  persistence only when the user requests it
- Provider-labelled active-flight, frequency, parking, and gate context without
  duplicating WyrmGrid Bridge simulator actuals
- User-previewed ACARS, crew, and gate actions with no ambiguous automatic retry
- Dispatch and SimBrief correlation without assuming SayIntentions imported the
  same plan
- VA-Link deferred until SayIntentions supplies a separate virtual-airline key
  and the submission, retention, and deletion rules are approved

### WyrmGrid Bridge

- Versioned, supervised sidecar protocol with explicit capabilities
  (implemented foundation)
- MSFS 2024 SimConnect detection and read-only telemetry as the primary provider
  (implemented; live certification and SimConnect client redistribution remain)
- Default-off provider auto-start and automatic recording, with explicit
  session retention and deletion controls
- Hoard flight-recording history with shared unit-aware altitude and speed
  graphs (implemented)
- WyrmChart telemetry sessions with gap-aware altitude, speed, fuel, attitude,
  and later planned-versus-actual graphs
- A thin MSFS in-simulator recording controller after an independent CommBus
  and package-distribution spike
- Flight lifecycle and planned-versus-actual summaries
- Explicit `.pln` export and flight-plan load after read-only telemetry is proven
- MSFS 2020 and FSUIPC compatibility providers
- X-Plane 12 Web API provider after the MSFS 2024 slice

### Navigation, networks, and automation

- Simulator-neutral `.pln` and `.fms` interchange and Little Navmap handoff
- OurAirports offline reference snapshot and identifier crosswalk
- Optional Navigraph Navdata after developer approval and entitlement design
- Optional VATSIM and IVAO Atlas layers with personal fields discarded
- Local notifications and iCalendar export; community delivery through explicit
  plugin capabilities

The complete sequence and provider constraints live in the
[external integrations programme](integrations/README.md).
The simulator UX sequence is detailed in the
[simulator experience roadmap](integrations/simulator-experience-roadmap.md).

## Later modules

- Dispatch and explainable job scoring using OnAir, SimBrief, SayIntentions.AI,
  weather, and simulator-neutral plan snapshots
- Operational Planner flagship plugin: Charter Desk and lease comparison first,
  followed by Airline Network scenarios after the public plugin surface is
  proven
- FBO network planning and coverage analysis
- Maintenance, finance, and flight history
- Hoard Timeline expansion across company value, geographic FBO coverage,
  routes, utilization, finance, and named milestones
- WyrmGrid Bridge flight lifecycle and planned-versus-actual use of the
  implemented telemetry foundation, followed by additional simulator and
  aircraft-specific providers
- Signed plugin packages, hardened supervision, and WyrmGrid Aerie discovery

Stable plugin APIs, automatic updates, signing, and a public plugin catalogue
require separate readiness reviews; they are not implied by the initial shell.

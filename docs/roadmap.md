# Roadmap

## Foundation

- Repository, governance, security, CI, and release automation
- Tauri/Svelte/MapLibre application shell
- Stable domain provenance and plugin manifest v1 groundwork
- Read-only credential-safe OnAir adapter and SQLite migration ownership

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
- Company connection plus initial fleet, aircraft, and airport translation
  (implemented); FBO translation remains
- Timestamped fleet refresh, compacted SQLite history, restart-time offline
  fallback, and explicit live/cached/offline state (implemented)
- Hoard Timeline fleet history: persistent LIVE/HISTORICAL mode indicator,
  as-of selection, return-to-present control, and fleet growth/composition
  charts
- Conservatively rate-protected manual and user-configured automatic fleet
  synchronization (implemented)
- Atlas fleet markers, layer toggle, map fitting, and linked aircraft inspector
  (implemented)

## Vertical slice 2: external plugin proof

- Process supervisor and framed protocol messages
- Permission review and persisted grants
- Fleet read and map layer publication capabilities
- Python SDK and idle-aircraft example plugin

## External operations track

This track begins after the fleet slice has SQLite persistence and safe offline
fallback. Its architecture may be prepared in parallel, but provider features do
not replace completion of the current vertical slice.

### Operational plan foundation

- Canonical provenance-aware flight-plan and route snapshots from ADR-0008
- Stable external provider ports and private raw response models
- AIRAC, identifier, unit, timestamp, freshness, and reconciliation policies
- Host-owned import/export validation and deny-by-default plugin capabilities

### SimBrief and weather

- Latest SimBrief OFP import by an explicitly connected Pilot ID or username
- OnAir aircraft, payload, airport, schedule, and deadline comparison
- Cached AviationWeather.gov METAR and TAF airport context
- Route advisories and explainable weather findings
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
- MSFS 2024 SimConnect detection and read-only telemetry as the primary provider
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

## Later modules

- Dispatch and explainable job scoring using OnAir, SimBrief, SayIntentions.AI,
  weather, and simulator-neutral plan snapshots
- Operational Planner flagship plugin: Charter Desk and lease comparison first,
  followed by Airline Network scenarios after the public plugin surface is
  proven
- FBO network planning and coverage analysis
- Maintenance, finance, and flight history
- Hoard Timeline expansion across company value, FBO footprint, routes,
  utilization, finance, and named milestones
- WyrmGrid Bridge simulator telemetry, followed by additional simulator and
  aircraft-specific providers
- Signed plugin packages and WyrmGrid Aerie discovery

Stable plugin APIs, automatic updates, signing, and a public plugin catalogue
require separate readiness reviews; they are not implied by the initial shell.

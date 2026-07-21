# Roadmap

## Foundation

- Repository, governance, security, CI, and release automation
- Tauri/Svelte/MapLibre application shell with lazy Three.js weather rendering
- Stable domain provenance and plugin manifest v1 groundwork
- Read-only credential-safe OnAir adapter and SQLite migration ownership
- Versioned first-run Terms, Privacy Notice, persistent user control, and
  opt-in privacy-filtered Rust and Svelte error monitoring (implemented);
  public Sentry operational review and native symbol upload remain
- Semantic styling, four built-in appearances, and a persisted, contrast-checked,
  data-only community theme manifest with local provenance, duplicate detection,
  export, deletion, and contrast-preview authoring (implemented); any curated
  distribution remains later ecosystem work
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
- Optional Windows credential-vault persistence for the OnAir API key,
  encrypted Company ID metadata, explicit Forget, and separately default-off
  connection at startup (implemented; other platform backends need release
  certification)
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
- Authenticated company-staff access gate, bounded translated roster,
  Hoard-backed live/cached/offline state, data-derived roster filters,
  accessible dossier drill-down, and a read-only Staff workspace (foundation
  implemented; provider enum labels, general certifications, and actual avatar
  artwork remain unavailable until their contracts are verified)

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

- Latest SimBrief OFP import by an explicitly supplied or optionally remembered
  Pilot ID or username, session-only plan interface, and explicit clear action
  (developer preview implemented; authenticated compatibility certification
  remains)
- OnAir aircraft identity, exact model-label, current-airport, selected-job
  route, reported cargo, and expiry comparison with unavailable evidence exposed
  instead of inferred (implemented for the current read-only contracts)
- Explicitly requested, ten-minute session-cached AviationWeather.gov METAR and
  TAF airport context through its independent provider plugin, with raw coded
  text and provenance (implemented)
- Host-owned plan-airport weather projection, linked Dispatch/Atlas station
  selection, explicit unknown/no-report rendering, and an initial
  Plan-to-Atlas journey rail (implemented)
- Route advisories and explainable weather findings (airport observations and
  privacy-preserving ETA-matched coarse global-model context implemented,
  with current-only legacy and factual RADAR context explicit; route hazard
  products remain)
- Dispatch-to-Atlas coordinate-only route projection with antimeridian-safe
  full-route framing, stable linked airport/fix selections, and explicit
  unresolved-location results (implemented); clickable SID/STAR geometry still
  waits for navigation and AIRAC resolution
- Live and Hoard-historical Atlas weather with selectable Compatibility,
  Enhanced, and Cinematic rendering, bounded WebGPU ray-marched volumes,
  WebGL2/MapLibre fallbacks, adaptive visual ceilings, and identical facts
  across rendering profiles (implemented)
- Independently approved Open-Meteo global model and RainViewer RADAR plugins
  with host-owned GPU rendering (six bounded Open-Meteo forecast horizons,
  coarse time-aware route matching, six-frame recent RADAR animation,
  timestamps, and visible no-data masks implemented; higher resolution,
  forecast confidence, and persisted history remain)
- SimBrief generation only after Navigraph approves the desktop flow and any
  required hosted-secret boundary receives a separate decision

### SayIntentions.AI

- Explicit opt-in read of the credential-bearing local active-flight payload
  through a reviewed fixed-loopback or `flight.json` transport
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
- Default-off provider auto-start and evidence-led automatic recording, with
  explicit session retention, pinning, export, and deletion controls
- Hoard flight-recording search, exact older/newer windows, lifecycle evidence,
  pinning, export, and shared unit-aware whole-flight altitude, speed,
  fuel-weight, and attitude graphs with bounded gap-safe downsampling
  (implemented)
- Versioned SimBrief planned-versus-recorded facts and labelled overlays for
  duration, track distance, altitude, fuel, airport proximity, and registration,
  plus historical planned/recorded Atlas route overlay (implemented v2)
- A thin MSFS in-simulator recording controller after an independent CommBus
  and package-distribution spike
- Additional lifecycle policies, phase analysis, and weather-along-track overlays
- Simulator weather-mode transitions and ambient conditions recorded separately
  from external real-world weather after an MSFS SDK and protocol spike
- Explicit `.pln` export and flight-plan load after read-only telemetry is proven
- MSFS 2020 and FSUIPC compatibility providers
- X-Plane 12 Web API provider after the MSFS 2024 slice
- Separately consented simulator-synchronised audio with user-selected codec
  providers and first-party Opus working tracks,
  encrypted segmented external media, metadata-only SQLite records, storage
  budgets, deletion, playback, and explicit export. The independent provider
  protocols, source/profile models, schemas, fixtures, deterministic fake
  capture, debug-only Windows microphone provider, and Opus codec sidecar are
  implemented; packaging and live certification remain unavailable.
- Windows/MSFS capability-labelled application, endpoint, or
  mixed-output capture; SimConnect COM facts remain metadata rather than
  isolated radio audio
- Cross-platform X-Plane microphone and mixed-output capture after the Web API
  telemetry provider, followed by isolated COM1/COM2 or pilot/copilot tracks
  only if the thin audio-tap stability and licensing spike succeeds
- Verified end-user codec-provider installation, signing, integrity, resource
  controls, updates, rollback, removal, trust presentation, and privacy notice

### Navigation, networks, and automation

- Simulator-neutral `.pln` and `.fms` interchange and Little Navmap handoff
- OurAirports offline reference snapshot and identifier crosswalk
- Optional Navigraph Navdata after developer approval and entitlement design
- Optional VATSIM and IVAO Atlas layers with personal fields discarded
- Local notifications and iCalendar export; community delivery through explicit
  plugin capabilities

### Core flight operation lifecycle

- Host-owned flight-operation identity, revision semantics, and a non-blocking
  Plan -> Weather -> Jobs -> Manifest -> Fleet -> Staff -> Review -> Atlas
  journey rail (schema-1 persistent identity, append-only revisions, current
  stage availability, and restart-safe active-operation view implemented)
- A per-leg manifest that distinguishes passengers, company personnel,
  positioning staff, the player avatar, and freight without double-counting
  people who operate one leg and travel on another (aggregate OnAir passenger
  counts, freight weights, and explicit missing facts implemented as the first
  evidence-derived slice; identities and roles remain)
- Explainable reconciliation across jobs, load, seats, payload, aircraft and
  staff location, sourced qualifications, weather, schedule, and plan evidence
  (read-only aircraft identity, model, current-airport, fleet-freshness, and
  manifest-coverage reconciliation plus append-only user-reviewed aircraft
  assignment implemented; certified seat, capacity, configuration, and
  availability evidence remain)
- Explicit invalidation and user-reviewed revisions instead of silent cascading
  changes when a plan, manifest, aircraft, staff assignment, or observation
  changes
- Atlas, Bridge recording, and Hoard debrief association with the accepted
  operation revision
- Future core Industry models for facilities, inventory, production, workforce,
  and logistics demand; Industry feeds flight operations rather than becoming a
  linear wizard step

The domain boundaries, staged sequence, privacy requirements, and current
evidence questions live in the
[flight operation lifecycle](operations/flight-operation-lifecycle.md).

The complete sequence and provider constraints live in the
[external integrations programme](integrations/README.md).
The simulator UX sequence is detailed in the
[simulator experience roadmap](integrations/simulator-experience-roadmap.md).

## External extension delivery track

This is a local application capability, not part of the optional hosted track.
It implements the invariant in
[ADR-0021](architecture/decisions/0021-externally-installable-extensions.md):
every plugin and provider can be delivered and managed as an external artifact
without rebuilding WyrmGrid.

- Ordinary plugin package schema version 1 now establishes the first canonical
  per-user installation root, bounded exact inventory, compatibility decision,
  inert unknown-format behaviour, staged Rust validation, explicit Forge trust
  review, immutable activation, provenance, disable, removal, update, and
  rollback.
- The four first-party Python plugins now build as deterministic, separately
  distributable packages and are seeded through that same public lifecycle.
- Simulator provider package schema version 1 now provides the same managed
  lifecycle for native Bridge sidecars while retaining a distinct executable
  trust class. The MSFS 2024 SimConnect provider is the deterministic reference
  `.wyrmprovider` and no longer depends on installer-only binary registration.
- Audio provider package schema version 1 now gives native audio sidecars their
  own `.wyrmaudio` contract, executable trust review, persistent selection, and
  the same install, disable, update, rollback, and removal lifecycle offered to
  community artifacts. The deterministic fake provider proves packaging and
  protocol conformance without claiming live native capture support.
- Prove offline manual installation, application upgrade compatibility,
  independent plugin failure, absent runtime/provider behaviour, and recovery
  from corrupt or permission-changing updates.
- Keep first-party seeding optional and make every seeded package separately
  distributable. A feature that cannot meet this boundary is documented as
  core, not as a plugin.

Local packages may be deliberately installed before Aerie exists, with their
unverified publisher and integrity status shown honestly. Recommendation of
unreviewed executable packages still waits for the relevant isolation,
conformance, integrity, update, and security gates.

## Hosted ecosystem track

This is an optional, separately approved track. It must not make application
startup, local Hoard access, existing plugins, local backup creation, or manual
installation of an already verified package depend on a WyrmGrid server. Its
architecture, detailed gates, and preliminary licence register are in
[ADR-0019](architecture/decisions/0019-hosted-web-aerie-and-private-vault.md),
the [hosted-platform implementation plan](operations/hosted-platform.md), and
the
[hosted-platform licensing and compliance register](legal/hosted-platform-licensing.md).

### Decisions and static website

- Confirm the legal operator, jurisdiction, final server and storage details,
  domain, DNS, operating system, backups, external providers, ongoing cost,
  availability, recovery and service-discontinuation expectations.
- Complete the package, signing, identity, privacy, legal, abuse, incident,
  dependency and hosted-migration decisions before implementing their phase.
- Begin with a reviewed, accessible, mostly static public site for project
  information, documentation, release verification guidance and security
  contact details, with no account, analytics, upload or private-data need.
- Prove clean-host reconstruction, external monitoring, off-site restore,
  deployment rollback, TLS renewal, dependency notices and safe outage
  behaviour before adding state.

### Curated read-only Aerie

- Define versioned catalogue, package-kind, repository-signing and desktop
  verification contracts with valid, invalid, malicious, expired, rollback,
  freeze, corrupt, yanked, revoked and permission-changing fixtures.
- Publish only reviewed immutable targets, beginning with lower-risk data-only
  assets. Keep downloads anonymous and installation explicitly user initiated.
- Keep the Rust service authoritative for compatibility, approval, revocation
  and installation; Svelte remains presentational.

### Publisher uploads and executable packages

- Add reviewed OpenID Connect, stable publisher identities, namespace and key
  recovery, narrowly scoped grants, named moderator accounts and append-only
  audit only after the curated pipeline is proven.
- Accept bounded objects into quarantine, validate them in isolated workers that
  cannot execute packages or access production credentials, and require human
  moderation before signed publication.
- Launch rights attestations, SPDX and notice collection, takedown, abuse,
  yanking, revocation, incident, off-site recovery and key-compromise processes
  before public submissions.
- Publish and recommend ordinary out-of-process executable plugins through
  Aerie only after protocol and SDK conformance, deny-by-default permissions,
  resource controls, client revalidation, and independent security review. The
  local staged installation and rollback path already belongs to the external
  extension track. Automatic updates remain a later decision.

### Optional private vault

- Consider only opaque storage of the existing client-encrypted `.wyrmbackup`
  after a separate privacy and security approval. Keep its authorization,
  database role, storage, backup, retention, support and incident boundaries
  separate from public Aerie data.
- Test password loss, corruption, cross-account access, replay, quota,
  generations, export, deletion, account closure, client-version restore,
  off-site recovery and provider loss before real user data is accepted.
- Defer record-level synchronization to a later ADR and versioned protocol with
  device keys, recovery, provenance, conflict, tombstone, deletion and
  mixed-version rules. Do not replicate the live SQLite database.

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
- WyrmGrid Bridge lifecycle and SimBrief correlation refinement, followed by
  additional simulator and aircraft-specific providers
- Hosted-ecosystem refinements that have passed the separate track's gates,
  including carefully scoped catalogue discovery and update ergonomics

Stable plugin APIs, automatic updates, signing, and a public plugin catalogue
require separate readiness reviews; they are not implied by the initial shell.

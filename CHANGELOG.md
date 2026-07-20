# Changelog

This file records notable changes to OnAir WyrmGrid. Every release explicitly
states whether it contains breaking changes; any breaking change requires a new
major application version.

## [Unreleased]

### New features

- Added data-only theme export, safe local deletion, visual duplicate
  detection, explicit bundled-versus-local provenance, and a complete theme
  authoring preview with live Rust-matched contrast thresholds. Manifest author
  fields remain visibly unverified, and the independently versioned theme
  schema remains at version 1.
- Added the non-shipping simulator-audio application slices: independently
  default-off master, manual, automatic, and source-specific consent; explicit
  permission requests; fake-provider orchestration; schema-18 metadata;
  purpose-keyed authenticated external segments; retention, pinned-session
  protection, tombstoned deletion, portable-backup omission, bounded
  authenticated packet inspection, and separately warned plaintext packet
  export. The deterministic fake remains a debug-only protocol tool: no
  microphone, native provider, audible playback, packaged capture, or live-
  support claim is enabled. The source language catalogue advances to version
  19 for the audio controls and recording-state labels.
- Added an optional, profile-driven local development-task framework for
  change-impact dossiers, test matrices, documentation synchronization,
  synthetic fixture variants, sanitized failure triage, and release curation.
  User-selected Ollama models and unauthenticated OpenAI-compatible loopback
  servers such as LM Studio, LocalAI, and llama.cpp receive only bounded,
  versioned, review-only handoffs. The runner captures exact reported token
  counts plus available timing and resource metadata; unavailable non-portable
  measurements remain explicit. WyrmGrid itself has no AI dependency.
- Added a deterministic `review:inventory` maintainer command with a strict
  version-1 evidence schema and sanitized fixture. It records NUL-safe Git
  state, repository-relative file metadata and SHA-256 hashes, stable candidate
  identities, conservative critical-path flags, and explicit unavailable facts
  beneath ignored local storage without running validation, using a network or
  model, caching results, or changing Git or tracked-file state.
- Added a provider-neutral generated-contribution provenance contract and
  fail-closed maintainer broker. A dedicated, least-privileged GitHub App can
  publish one hash-bound bot commit and branch for a wholly local-assistant-
  generated text patch; the human maintainer then opens its draft PR. The App
  private key remains isolated and the App receives no Pull requests, review,
  merge, release, workflow, or protected-path authority.
- Added durable, revisioned flight operations built from an imported SimBrief
  plan, an optional read-only OnAir job, and retained manifest evidence.
- Added read-only fleet reconciliation to accepted flight operations. WyrmGrid
  derives a deterministic registration or unique-model candidate from the
  current OnAir fleet, compares its model and airport with the accepted plan,
  reports manifest coverage and fleet freshness, and keeps unverified seats,
  payload capacity, configuration, and operational availability explicitly
  unavailable rather than treating the candidate as an assignment.
- Added independent first-party AviationWeather.gov, Open-Meteo, and RainViewer
  provider plugins with bounded weather requests and publications.
- Added sourced airport, global-model, and radar weather presentation in Atlas,
  including persistent Enhanced GPU and Compatibility rendering preferences.
- Added Cinematic Atlas weather graphics with layered source-shaped clouds,
  visible rain and snow, convective lightning illumination, dust and sand
  effects, independent phenomenon controls, flash reduction, and automatic
  low-resource and Reduced Motion fallbacks.
- Added a lazy Three.js weather renderer that prefers WebGPU, falls back to the
  Three.js WebGL2 backend, and restores the existing MapLibre effects after
  initialization, rendering, or graphics-device failure. Initial visible-cell,
  particle, cloud, dust, and resolution ceilings keep Enhanced and Cinematic
  work bounded while preserving all sourced markers and provenance.
- Added bounded TSL ray-marched WebGPU density volumes for source-local clouds,
  obscuration, and dust. A deterministic shared 3D texture, WebGL2 mesh/point
  substitutes, and conservative in-memory pressure levels enrich weather while
  preserving the selected profile, factual layers, and failure fallback.
- Added projection-aware Atlas weather composition that fades decorative cells
  behind the globe or pitched-map horizon using a host projection round trip.
  Screen-stable stratified volume sampling reduces ray-march banding without
  introducing frame-varying noise or claiming shared terrain depth.
- Added a toggleable UTC day/night layer with civil, nautical, and
  astronomical twilight, including historical-Hoard time projection. Added a
  separate weather-support-zone layer for indicative airport observation
  vicinity, compact source-centred model footprints, and exact received RADAR tile
  footprints without presenting point observations as storm boundaries or
  allowing sparse samples to colour continent-sized cells.
  Theme-independent repeating patterns distinguish cloud, rain, snow,
  convective, obscuration, dust, and RADAR zones without relying on colour
  alone.
- Added a per-plugin **Start automatically with WyrmGrid** choice. It is off by
  default, requires standing access, survives ordinary restarts, and becomes
  inactive whenever the plugin version, capabilities, weather products, or
  approved network origins change.
- Added a guarded **Erase the WyrmGrid database** control to Encrypted data &
  backups. It requires an acknowledgement and exact typed phrase, restarts the
  application, and replaces the active database with an empty encrypted one
  while leaving portable backups, installed plugins, diagnostics, simulator
  sidecars, browser-webview local storage, and separately stored operating-system
  credentials alone.
- Added encrypted Atlas continuity preferences for automatic synchronization
  and layer visibility, plus an opt-in **Restore my last Atlas view** choice
  that clears its bounded camera values when disabled.
- Added host-owned Forge settings with bounded refresh choices for forecast-grid
  and RADAR plugins. Plugins cannot declare, read, or write these non-secret
  records, and plugin API version 1 remains unchanged.
- Added a bounded six-frame RainViewer RADAR timeline with visible source
  timestamps, play/pause and stepping controls, automatic motion-safe static
  fallback, and provider no-coverage masks rendered distinctly from clear sky.
- Added Rust-owned weather-along-route analysis for continuous mapped plan
  segments. Dispatch now matches coarse global-model checkpoints to
  proportional planned ETAs within explicit spatial and temporal limits,
  labels legacy data as current-only context, and shows the latest factual
  RADAR timestamp without extrapolation. Atlas distinguishes ETA-matched,
  current-only, unsupported, and unresolved corridor sections without a safety
  score.
- Added on-demand historical weather for imported past SimBrief plans. Dispatch
  requests bounded actual METAR observations and Open-Meteo Historical Forecast
  model samples, keeps the result session-local, excludes current forecast and
  RADAR layers, and labels the complete Atlas/Dispatch presentation historical.
- Documented a near-future, optional ActiveSky weather plugin with explicit
  simulator-weather provenance, absence-safe behavior, narrow local transport,
  renewed consent, and no assumption that a SimBrief plan carries ActiveSky's
  full weather field.

### Changes

- Unified the local formatting commands across Rust, frontend, and documentation
  files. Rust formatting now uses the repository's LF newline policy on every
  operating system, so `npm run format` repairs the recurring Windows mismatch
  deterministically without an AI or hosted runner.
- Replaced rectangular global-model support patches with compact, source-centred
  circular footprints while preserving the clearer solid dispatch route. Their
  radius is capped and constrained by model-grid spacing; they remain indicative
  sample support rather than measured weather-pattern boundaries. A provider may
  vary a footprint only through an explicit, validated first-party extent;
  intensity and cloud or precipitation values never invent pattern size.
- Extended plugin API version 1 with an additive bounded historical UTC window
  and exact historical-layer time scope. Live requests remain unchanged, while
  an unaware plugin's current response fails closed instead of being accepted
  under a historical label.
- Advanced the bundled Open-Meteo provider to 0.3.0 with its separately
  allowlisted Historical Forecast origin and the AviationWeather.gov provider
  to 0.2.0 with windowed historical METAR selection. Provider scope changes
  invalidate previous standing grants and automatic-start choices until the
  user reviews them; Open-Meteo explicitly requests its additional origin.
- Kept Python plugin HTTPS certificate and hostname verification mandatory
  while using the Windows operating-system root store instead of a potentially
  stale standalone OpenSSL CA file.
- Fixed the bundled Open-Meteo six-horizon product being rejected by the host's
  legacy one-point correlation check. Strict additive horizon correlation keeps
  every returned sample tied to an exact host-selected location while preserving
  plugin API version 1 and legacy one-to-one products.
- Routed background plugin-supervisor failures into the bounded local
  Diagnostics log with stable reason codes, lifecycle operations, controlled
  messages, and validated local plugin identifiers. Desktop command, startup,
  partial-sync, and plugin diagnostics now share one reporting broker;
  unexpected failures reuse the existing consent-gated, redacted Sentry adapter
  with stable codes only. Plugin identifiers, output, payloads, URLs, and
  provider data remain local or excluded. Privacy Notice version 2026-07-19.3
  records the new local field.
- Updated the Rust Sentry SDK to 0.48.5 for its panic-handler safety fix and the
  SvelteKit SDK to 10.66.0 for current browser and SvelteKit fixes; the Vite
  upload plugin was already current at 5.4.0. Error-only settings and telemetry
  minimisation remain unchanged.
- Existing webview synchronization choices now migrate once into encrypted
  settings and are removed from browser storage after a successful save. The
  privacy notice and source language catalogue advance for the newly retained
  Atlas and plugin preferences.
- Extended the additive plugin API version-one weather shape with a bounded
  RainViewer-only recent-frame offset and optional per-tile coverage PNG. The
  legacy third-party request shape and existing limits remain unchanged; the
  source language catalogue advances to version 15 for RADAR controls,
  no-data wording, and route-weather presentation.
- Advanced the bundled RainViewer provider to 0.2.0 and made reserved bundled
  plugin files refresh from the installed application. The provider-version
  change invalidates its previous standing grant and automatic-start choice
  until the user reviews and approves the current provider scope.
- Advanced the bundled Open-Meteo provider to 0.2.0 and changed its fixed
  84-location request to six bounded UTC forecast horizons. Optional per-point
  valid times remain additive in plugin API version 1; existing third-party
  points continue to work as explicitly labelled current context. The provider
  version change invalidates its previous standing grant and automatic-start
  choice until the user reviews the current provider scope.
- Advanced the source language catalogue to version 16 for planned route ETA,
  forecast-validity, current-context, and observation-only RADAR wording.
- Advanced the source language catalogue to version 17 for explicit historical
  route-weather and Atlas wording.
- Advanced the source language catalogue to version 18 for theme provenance,
  lifecycle controls, authoring, contrast-preview, and duplicate guidance.
- Advanced the source language catalogue to version 19 for audio consent,
  source state, recording state, authenticated inspection, export, deletion,
  and backup/reset wording.

- Documented a proposal-only hosted-platform architecture and staged delivery
  plan for the public website, WyrmGrid Aerie catalogue, signed community
  packages, and a separately isolated encrypted-backup vault. Added a candidate
  software-licence register, community-package rights policy, operating-cost
  ledger, threat controls, and legal gates. No server deployment, account,
  upload, private-data service, package format, signing key, version, workflow,
  or public support claim is enabled.
- Documented a staged, proposal-only plan for release-integrity hardening,
  truthful pull-request validation, deterministic release gates, protected
  GitHub promotion, supply-chain evidence, platform coverage, and optional
  local Hoardmind review conveniences. The plan authorizes no workflow, GitHub
  setting, version, tag, release, cache, secret, or signing change.
- Documented a proposal-only local review-automation programme that uses
  deterministic change evidence, validation receipts, bounded packet
  preparation, and incremental caching to support comments, tests, fixtures,
  documentation, failure triage, boundary audits, and release-readiness
  evidence. Hoardmind remains optional and review-only; ChatGPT/Codex semantic
  review remains mandatory for high-benefit and critical work. Stage 1 now
  implements only the deterministic inventory; later helpers, model authority,
  workflows, and release behaviour remain unimplemented.
- Documented a proposal-only implementation programme for the selected
  SayIntentions, direct VATSIM, direct IVAO, and Navigraph integrations. It
  defines provider-specific sequencing, privacy and authorization gates,
  unavailable-data behaviour, validation, and live-certification requirements
  without enabling provider access or claiming compatibility.
- Reconciled maintainer-supplied SayIntentions public-document screenshots with
  the current provider page and recorded the SAPI endpoint inventory,
  `flight.json` field decisions, SimAPI boundary, immediate subscribed-account
  path, VA-Link exception, and sanitized live-certification requirements.
- Reworked detailed Atlas cloud, storm, obscuration, and dust volumes with a
  higher-resolution multi-lobed density field, stable optical thickness,
  brighter internal shading, sourced-wind alignment, and deterministic
  three-dimensional variation. Rain and snow now begin below their parent
  cloud and taper around the precipitation field instead of surrounding its
  centre.
- Added an isolated deterministic weather gallery for renderer calibration and
  hardware diagnosis. It is available automatically in development builds and
  in packaged builds only through the explicit `--weather-gallery` startup
  flag; its controls are temporary and never replace supported user graphics
  preferences.
- Fixed detailed Atlas weather exposing the hard faces of its volume boxes,
  repeating recognisable cloud silhouettes, crossing the globe horizon, and
  drawing over map information cards.
- Added compile-time catalogue-key ownership for translated interface code,
  replaced constructed Settings and Simulator message identifiers with
  explicit typed mappings, and moved Atlas renderer status text into source
  catalogue version 12.
- Added deterministic localization and desktop-command identifier audits, plus
  a repository-wide boundary report covering current UI, Tauri, Atlas,
  localization, migration, protocol, and provider separation.
- Reserved Codex semantic review of valid Hoardmind output for high-benefit or
  critical work. Lower-benefit drafts now avoid redundant frontier-model review
  while deterministic gates and mandatory human release or generated-
  contribution approvals remain unchanged.
- Documented the accepted future simulator-synchronised audio architecture:
  Opus working tracks, capability-labelled MSFS and X-Plane sources, separate
  audio consent, encrypted external media, metadata-only SQLite records, and an
  independently versioned provider boundary. No audio capture implementation is
  included yet.
- Generated-contribution squash landing now uses a human-authenticated,
  exact-head guard that verifies the App-bot commit and clean protected PR,
  supplies every provenance trailer explicitly, forbids administrative bypass,
  and verifies the resulting merge commit before reporting success.
- Jobs opened from Dispatch now default to the imported plan's exact route and
  preserve an explicit path back to all pending jobs.
- Atlas, Jobs, Dispatch, Flight Operations, Settings, Security, Simulator,
  Staff, Forge, Legal, Diagnostics, and Data Protection now adapt their layout
  to narrow or short windows without changing business behaviour.
- Weather acquisition moved from the application process to supervised Python
  provider plugins. The new protocol-v1 messages and capabilities are additive,
  so existing version-one plugins remain compatible; weather providers require
  Python 3 and explicit user approval and fail independently when unavailable.
- Database migrations 0013 through 0015 append flight-operation history and
  Atlas weather preferences without rewriting an earlier released migration.
  Migration 0015 retains the earlier two-profile preference as a legacy
  fallback while making the richer graphics record authoritative.
- Database migration 0016 adds host-owned, scope-bound plugin startup
  preferences without changing plugin protocol version 1. Source catalogue
  version 13 adds the associated Forge and destructive-reset wording.

### Removed

- Removed the internal in-process `wyrmgrid-weather-api` crate after its
  provider-specific responsibilities moved to first-party weather plugins.

### 🚨 Breaking changes

- None.

## [0.2.0] - 2026-07-17

### New features

- Added whole-flight Hoard debrief graphs, bounded long-recording downsampling,
  fuel and attitude evidence, planned-versus-recorded comparisons, and recorded
  Atlas routes.
- Added Atlas administrative regions, zoom-sensitive detail, route projection,
  historical tracks, and sourced airport-weather context.
- Added a read-only OnAir Staff workspace with verified qualifications and
  shared search, filtering, sorting, result counts, and drill-down patterns.
- Added optional remembered OnAir credentials and SimBrief Pilot ID storage
  using the operating-system credential service and encrypted local storage.
- Added once, session, and standing permission lifetimes to the deny-by-default
  authorization model.

### Changes

- Improved responsive presentation, dialog navigation history, interaction
  preferences, and shared collection exploration across operational workspaces.
- Strengthened tagged release checks, checksum handling, draft publication, and
  Windows in-place upgrade verification while preserving application identity
  and existing encrypted data.

### Removed

- None.

### 🚨 Breaking changes

- None.

## [0.1.0] - 2026-07-15

### New features

- Established the Rust, Tauri, Svelte, MapLibre, and declarative-chart desktop
  foundation with stable domain models and provenance-aware application layers.
- Added session-only, read-only OnAir synchronization for fleets, FBOs, and
  pending jobs with rate protection and credential-safe error handling.
- Added Atlas fleet and FBO views plus encrypted offline Hoard snapshots, fleet
  history, and the Hoard Timeline.
- Added read-only Jobs-to-Dispatch comparison, SimBrief latest-plan import, and
  explainable airport-weather context.
- Added the supervised out-of-process plugin runtime, deny-by-default
  capabilities, and the first data-only Atlas plugin proof.
- Added community localization, safe themes, legal and privacy onboarding, and
  optional privacy-reduced observability foundations.
- Added the isolated MSFS 2024 SimConnect Bridge, telemetry recording, explicit
  disconnect gaps, measurement-unit settings, and safe unavailable behaviour.
- Added core authorization, Security Centre, SQLCipher-encrypted storage,
  operating-system-protected device keys, and password-encrypted portable
  backup and restore.
- Added tagged CI release automation with platform packages, checksums, and
  draft-prerelease publication controls.

### Changes

- None.

### Removed

- None.

### 🚨 Breaking changes

- None.

[Unreleased]: https://github.com/phobos-dthorga/onair-wyrmgrid/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/phobos-dthorga/onair-wyrmgrid/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/phobos-dthorga/onair-wyrmgrid/releases/tag/v0.1.0

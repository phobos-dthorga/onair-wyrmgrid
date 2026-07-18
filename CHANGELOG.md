# Changelog

This file records notable changes to OnAir WyrmGrid. Every release explicitly
states whether it contains breaking changes; any breaking change requires a new
major application version.

## [Unreleased]

### New features

- Added an optional, profile-driven local development-task framework for
  change-impact dossiers, test matrices, documentation synchronization,
  synthetic fixture variants, sanitized failure triage, and release curation.
  User-selected Ollama models and unauthenticated OpenAI-compatible loopback
  servers such as LM Studio, LocalAI, and llama.cpp receive only bounded,
  versioned, review-only handoffs. The runner captures exact reported token
  counts plus available timing and resource metadata; unavailable non-portable
  measurements remain explicit. WyrmGrid itself has no AI dependency.
- Added a provider-neutral generated-contribution provenance contract and
  fail-closed maintainer broker. A dedicated, least-privileged GitHub App can
  publish one hash-bound bot commit and branch for a wholly local-assistant-
  generated text patch; the human maintainer then opens its draft PR. The App
  private key remains isolated and the App receives no Pull requests, review,
  merge, release, workflow, or protected-path authority.
- Added durable, revisioned flight operations built from an imported SimBrief
  plan, an optional read-only OnAir job, and retained manifest evidence.
- Added independent first-party AviationWeather.gov, Open-Meteo, and RainViewer
  provider plugins with bounded weather requests and publications.
- Added sourced airport, global-model, and radar weather presentation in Atlas,
  including persistent Enhanced GPU and Compatibility rendering preferences.

### Changes

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
- Database migrations 0013 and 0014 append flight-operation history and Atlas
  weather preferences without rewriting an earlier released migration.

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

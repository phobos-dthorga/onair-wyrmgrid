# Observability and Sentry plan

This document turns
[ADR-0007](../architecture/decisions/0007-hosted-sentry-observability.md)
into an implementation and operating plan. Account names, DSNs, organisation
slugs, and authentication tokens are deliberately absent.

The latest bounded code and dependency review is recorded in the
[2026-07-19 observability audit](observability-audit-2026-07-19.md).

## Initial hosted layout

Use one Sentry Cloud organisation in the U.S. data region with these projects:

| Project                 | Runtime                               | Initial signals                                   |
| ----------------------- | ------------------------------------- | ------------------------------------------------- |
| `wyrmgrid-desktop-rust` | Rust/Tauri process                    | Panics and selected unexpected errors             |
| `wyrmgrid-desktop-ui`   | SvelteKit client in the Tauri webview | Unhandled interface errors                        |
| `wyrmgrid-website`      | Future static website                 | Deferred until the site has meaningful JavaScript |

The Developer plan is appropriate while there is one maintainer and low event
volume. Reassess the Team plan when another maintainer needs access, integrated
alerts become useful, or filtered production errors consistently approach the
plan quota. Set a hard pay-as-you-go budget before enabling any paid overage.

## Implemented foundation

- Serializable operation errors carry a stable code, safe message,
  retryability, reportability, and optional report identifier.
- Frontend commands pass through one typed desktop client.
- Rust and SvelteKit adapters require an explicit enable flag, a configured
  DSN, and the current versioned user preference.
- The preference is off by default, persisted locally in SQLite, and available
  from **Privacy & Terms** after first-run acknowledgement.
- Both adapters remove user, request, breadcrumb, extra, local-path,
  source-line, and unapproved context data before sending.
- Expected input, authentication, rate-limit, offline, and optional-integration
  conditions are not reported.
- Release CI can upload Svelte source maps using protected GitHub configuration.
- A separate local diagnostic log retains at most 200 structured English entries
  in `wyrmgrid-diagnostics.jsonl` under the application-data directory. It is
  readable and clearable from **Diagnostics** in the desktop header.
- Desktop command, background-startup, partial-sync, and plugin-supervisor
  diagnostics pass through one vendor-neutral reporting broker. It always
  writes the bounded local entry and invokes the Sentry adapter only when the
  application-owned error classification is reportable. A plugin entry may
  include one validated manifest plugin ID locally, but only its stable code can
  reach the consent-gated Sentry adapter; plugin IDs, output, payloads, URLs,
  and provider data are not added to Sentry events.

## Local diagnostic log

The local log is available whether or not Sentry telemetry is configured or
enabled. It records stable error codes, controlled English messages, operation
names, severity, timestamps, and an optional validated plugin manifest ID. It
does not record raw provider responses,
request URLs, headers, OnAir API keys, company identifiers, domain snapshots,
plugin output, or user-entered text. Pending-job synchronization failures are
recorded even when fleet or FBO synchronization succeeds and the combined
operation returns partial data.

Diagnostic wording is deliberately outside the community localization system.
Stable English codes and messages make support evidence comparable across
installations and avoid an untrusted language pack changing the apparent cause
of a failure. The user-facing workflow may still present localized recovery
guidance separately.

The file rotates to the most recent 200 entries and can be cleared in the
interface. WyrmGrid never uploads or attaches this file automatically. A user
should review entries before sharing them even though the writer accepts only
bounded application-owned fields.

| Kind     | Name                  |
| -------- | --------------------- |
| Variable | `SENTRY_ORG`          |
| Variable | `SENTRY_RUST_PROJECT` |
| Variable | `SENTRY_UI_PROJECT`   |
| Secret   | `SENTRY_AUTH_TOKEN`   |
| Secret   | `SENTRY_RUST_DSN`     |
| Secret   | `SENTRY_UI_DSN`       |

The DSN secrets are reserved for deliberately enabled builds; current public
release jobs do not embed them. Before public telemetry, verify server-side PII
and IP scrubbing, one sanitized event from each runtime, retention and access
roles, quota alerts and a hard overage budget, native debug-information upload,
and the current data-processing and international-transfer terms. The complete
gate is maintained in [Legal and privacy readiness](../legal/readiness.md).

## Phase 0: architecture preparation

- Replace string-only Tauri failures with a serializable command error carrying
  a stable code, safe message, retryability, and optional report identifier.
- Preserve privacy-safe diagnostic categories through application and adapter
  layers instead of collapsing sources into `String`, `Option`, or
  `Result<(), ()>`.
- Make storage and cache degradation explicit, including persistent,
  memory-only, unavailable, incompatible, and corrupt states.
- Route frontend invocations through one typed client and detect browser preview
  explicitly rather than interpreting every failed command as preview mode.
- Define one canonical build-information contract for application version,
  commit, release channel, operating system, and architecture.
- Add unit tests that place synthetic credential and company identifiers into
  candidate events and prove that redaction removes them.

Phase 0 introduces no network telemetry and should precede larger application,
plugin, or sidecar expansion.

## Phase 1: error-only integration

- Initialize the Rust SDK before Tauri startup and retain its client guard until
  the event loop exits.
- Initialize the official SvelteKit SDK in the earliest client hook.
- Keep both SDKs disabled when their DSN or explicit runtime setting is absent.
- Configure `send_default_pii`/`sendDefaultPii` as false and disable IP storage
  in Sentry project settings.
- Install `before_send`/`beforeSend` and breadcrumb filters backed by pure,
  unit-tested redaction functions.
- Add only the exact provisioned ingestion origin to the webview's
  `connect-src` content-security policy.
- Capture a synthetic Rust error and interface error from a maintainer build;
  verify project routing, release, environment, tags, redaction, and grouping.
- Do not capture the raw `reqwest::Error`, URL, HTTP body, SQLite payload,
  filesystem path, or plugin output. Emit bounded diagnostic codes instead.

## Phase 2: release correlation and symbolication

- Use `onair-wyrmgrid@<semver>` in both Sentry projects.
- Upload Svelte/Vite source maps during CI installer builds and delete them from
  distributable assets after upload.
- Keep `SENTRY_AUTH_TOKEN` and equivalent upload credentials in GitHub Actions
  secrets, never in source, local examples, application bundles, or logs.
- Produce and upload the appropriate PDB, dSYM, or ELF debug information before
  stripping published binaries.
- Associate releases with the Git commit and verify one deliberately generated
  stack trace on every supported platform before stable release.
- Leave a release draft if required diagnostic artifacts fail to upload; either
  repair the upload or record an explicit exception before publication.

## Phase 3: public-release control

- Add a concise privacy disclosure explaining what is collected, why, where it
  is stored, retention, and how to disable it.
- Provide a user-visible telemetry preference before public builds transmit
  events.
- Never attach the Hoard database, screenshots, local logs, plugin output, or
  crash dumps automatically.
- Document a support workflow that accepts a Sentry report identifier without
  asking users to disclose protected local data.

## Deferred signals

Performance tracing may later sample safe operation names such as application
startup, fleet synchronization, snapshot load, and plugin lifecycle. Span names
and tags must remain low-cardinality and must not contain company, aircraft,
airport, route, coordinate, path, or credential values. Rust
`#[instrument]` functions must explicitly skip every sensitive argument.

Logs, Session Replay, profiling, user feedback, screenshots, attachments, and
native minidumps remain off until each has a documented use case, collection
schema, redaction tests, retention decision, and user-facing disclosure.

## Safe event vocabulary

Initially allow only:

- release, environment, operating system, architecture, and application module;
- stable operation and error codes;
- storage mode and data availability category;
- synchronization trigger and retryability;
- plugin or sidecar lifecycle category without untrusted output.

Plugin identifiers remain local diagnostic context and are not part of the
initial Sentry vocabulary.

Never send:

- OnAir API keys or credential-like values;
- company IDs or names;
- aircraft identifiers, registrations, airports, routes, or coordinates;
- SimBrief Pilot IDs, usernames, plan IDs, OFP contents, AIRAC entitlements, or
  provider download URLs;
- SayIntentions API keys, account IDs, email, flight IDs, `flight.json` fields,
  request URLs, communications, audio links, gates, runways, frequencies,
  multiplayer settings, or generated message text;
- Navigraph, VATSIM, or IVAO member identifiers, callsigns, network flight
  plans, ATIS text, positions, or free-form remarks;
- weather report text, advisory geometry, simulator values, tracks, flight-plan
  files, sidecar messages, or localhost provider responses;
- raw OnAir JSON, HTTP bodies, headers, or request URLs;
- SQLite payloads, local paths, usernames, or machine names;
- free-form plugin, simulator, or user-provided content.

## Operational review triggers

Review hosting and plan choice when any of these occur:

- a second maintainer needs Sentry access;
- monthly filtered errors approach the current plan quota;
- a paid integration has a demonstrated operating value;
- a legal, contractual, data-residency, or air-gap requirement changes;
- sustained managed-service cost exceeds a realistic self-hosted total cost;
- telemetry noise or cardinality makes alerts unreliable.

Review current Sentry pricing and self-hosted requirements at the time of each
decision rather than treating the 2026-07-14 plan limits as permanent.

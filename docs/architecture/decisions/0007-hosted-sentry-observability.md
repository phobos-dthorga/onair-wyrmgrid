# ADR-0007: Hosted Sentry behind observability adapters

- Status: Accepted
- Date: 2026-07-14

## Context

WyrmGrid needs actionable diagnostics from its Rust/Tauri process and embedded
Svelte interface as public use grows. Failures may occur in the webview,
application services, storage, network adapters, plugin supervision, simulator
sidecars, and release/update machinery. Plain user-facing strings and silent
fallbacks do not preserve enough safe context to diagnose those failures.

Sentry currently has separate supported Rust and JavaScript SDKs but no
first-party Tauri SDK. Self-hosted Sentry also carries a substantial fixed
operational cost: its documented minimum is four CPU cores, 16 GB RAM plus
swap, and storage with significant I/O, with 32 GB RAM recommended. That cost
is disproportionate for a single-maintainer prerelease project.

Observability is also a confidentiality boundary. OnAir credentials, company
identifiers, operational history, locations, local paths, database contents,
and untrusted plugin output must not enter telemetry merely because a library
can automatically collect them.

## Decision

- Begin with Sentry Cloud's Developer plan rather than self-hosted Sentry.
- Use the U.S. data region selected for the initial organisation. Revisit the
  region only if a concrete legal, contractual, or data-residency requirement
  changes; document any migration separately.
- Use separate Sentry projects and DSNs for the Rust desktop process and the
  Svelte desktop interface. A future public website receives a third project
  only when it has meaningful client-side behaviour to monitor.
- Give all WyrmGrid components for a shipped build the same canonical release
  name, `onair-wyrmgrid@<semver>`, with platform and architecture represented
  as distributions or tags.
- Keep Sentry dependencies at composition boundaries. Domain, application,
  storage, OnAir, and plugin-protocol contracts must not expose Sentry types.
  They return typed outcomes and may emit carefully designed vendor-neutral
  `tracing` events.
- Add a Rust observability adapter at desktop startup and a browser
  observability adapter at the SvelteKit client entry point. Telemetry failure
  must never prevent startup, command completion, offline use, or shutdown.
- Begin with error monitoring only. Performance tracing, logs, profiling,
  Session Replay, user feedback, screenshots, attachments, and native crash
  dumps require separate privacy and value reviews before activation.
- Disable default PII collection and IP storage. Events and breadcrumbs pass
  through explicit client-side allowlisting/redaction, with Sentry-side
  scrubbing as defence in depth.
- Public builds do not transmit telemetry until WyrmGrid has a user-visible
  disclosure and control. Maintainer and prerelease test builds may enable it
  deliberately without changing production defaults.
- Keep Sentry upload credentials only in CI secret storage. A DSN is a public
  submission endpoint rather than an account credential, but account-specific
  configuration still belongs at the build/deployment boundary.
- Upload JavaScript source maps and native debug information from CI release
  builds. Do not publish CI authentication tokens or unintentionally include
  source maps in application packages.

## Reporting policy

Unexpected defects are reportable: panics, unhandled interface exceptions,
poisoned state, storage initialization or migration failures, corrupt snapshots,
serialization failures, protocol violations, and unexpected command, plugin,
sidecar, or updater failures.

Expected conditions are not Sentry issues: invalid input, empty credentials,
authentication rejection, company-not-found responses, rate limiting, normal
offline operation, unavailable optional simulators, and intentionally ignored
duplicate synchronization. Some may become redacted breadcrumbs or aggregate
health counters after a separate review.

## Consequences

WyrmGrid gains managed ingestion, alerting, release correlation, and
symbolication without operating another production service. The initial free
plan can be upgraded without changing domain architecture. Sentry outages or
quota exhaustion result only in missing telemetry.

Two SDKs mean frontend and Rust failures are not automatically one distributed
trace. Stable error codes, optional report identifiers, and canonical release
metadata provide correlation without coupling application contracts to Sentry.

Self-hosting may be reconsidered for an air-gapped or contractual requirement,
materially different data-control needs, or sustained volume that makes managed
hosting uneconomic after infrastructure and maintainer time are counted. Growth
alone is not sufficient justification.

## References

- [Sentry pricing](https://sentry.io/pricing/)
- [Self-hosted Sentry requirements](https://develop.sentry.dev/self-hosted/)
- [Sentry DSN guidance](https://docs.sentry.io/concepts/key-terms/dsn-explainer/)
- [Sentry JavaScript source-map uploads with Vite](https://docs.sentry.io/platforms/javascript/guides/svelte/sourcemaps/uploading/vite/)

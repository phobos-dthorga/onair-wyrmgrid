# Observability audit — 2026-07-19

## Scope and method

This bounded local audit traced the Rust/Tauri and SvelteKit error paths,
telemetry consent, build gates, redaction, local Diagnostics persistence,
plugin-supervisor failures, dependency versions, privacy wording, and threat
controls. It did not connect to a Sentry project, transmit an event, inspect a
live account, enable a DSN, change a release workflow, or run the repository-wide
AI security scanner.

After deterministic root-cause discovery, the maintainer's optional private
local assistant produced separate change-impact and test-matrix drafts through
the versioned local handoff runner. Exact local usage and the review-only output
remain outside the repository. The packets contained no credentials, raw
provider payloads, or personal data; no draft was chained into another model
task or applied automatically. Suggestions that did not match the source were
discarded, while the useful 504-point, ambiguous-prefix, first-failure, code-
boundary, and old-log compatibility cases were verified independently.

## Outcome

The existing design remains appropriate: separate Rust and interface projects,
error-only collection, explicit current user consent, an additional build-time
enable switch, a configured DSN, client-side allowlisting, and server-side
scrubbing gates. Performance traces, logs, replay, profiling, feedback,
screenshots, attachments, breadcrumbs, and default PII remain disabled.

The main defect was coverage rather than an obsolete architecture. Tauri command
errors already entered the local diagnostic log and sent reportable stable codes
through the Rust adapter, but asynchronous plugin-supervisor failures stored only
a user-facing runtime string. They therefore bypassed both Diagnostics and
Sentry. The plugin supervisor now emits vendor-neutral, bounded events through an
observer owned by the desktop composition layer. That observer and the existing
command, startup, and partial-sync producers now share one desktop reporting
broker, so local persistence and the reportability decision cannot drift across
parallel call paths.

## Implemented controls

- A failed runtime emits at most one event containing a validated manifest
  plugin ID, stable host-owned code, lifecycle operation, fixed English message,
  severity, and reportability decision.
- Every desktop diagnostic producer now enters the same broker. It records the
  local entry first and calls Sentry only for an explicitly reportable
  application-owned classification; expected partial-provider and automatic
  plugin-start failures remain local-only.
- The plugin ID is retained only in the rotating local log and is visible and
  searchable in **Diagnostics**. Plugin output, payloads, URLs, weather values,
  provider bodies, paths, and user-entered content are not recorded.
- The Sentry adapter receives only the stable code. It remains inactive unless
  current legal acknowledgement, explicit user choice, the build flag, and the
  runtime-specific DSN all permit it.
- Both Rust and TypeScript redaction filters now validate the `error.code` value,
  not merely its tag name. Non-machine-owned or overlong values are removed.
- Expected weather timeouts and startup handshakes remain local-only. Protocol
  violations, structurally invalid products, poisoned supervisor state, and
  unexpected process loss remain reportable under ADR-0007.

## Dependency review

- Rust `sentry` advanced from 0.48.4 to 0.48.5. The upstream patch fixes a case
  where the SDK could panic while handling a panic:
  <https://github.com/getsentry/sentry-rust/releases/tag/0.48.5>.
- `@sentry/sveltekit` advanced from 10.65.0 to 10.66.0. The upstream release
  includes current browser fixes and SvelteKit detection changes:
  <https://github.com/getsentry/sentry-javascript/releases/tag/10.66.0>.
- `@sentry/vite-plugin` was already current at 5.4.0. Its own telemetry remains
  disabled in WyrmGrid's build configuration.
- `npm audit` still reports one low-severity `cookie` advisory through SvelteKit,
  reflected against the direct SvelteKit, static adapter, and Sentry wrapper
  packages. No non-breaking fix is currently available. WyrmGrid uses the static
  Tauri client rather than SvelteKit's server cookie-serialization path. Keep the
  advisory monitored and re-run the audit when SvelteKit publishes a fixed
  dependency resolution; do not force the suggested incompatible adapter
  downgrade.

## Local validation

- `cargo test --workspace` passed. Focused application coverage passed all 132
  tests, including the 84-by-6 Open-Meteo regression and hostile-correlation
  cases; focused desktop Rust coverage passed all 22 tests.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo deny check` passed. `cargo audit` reported only the 17 warnings already
  allowed by repository policy.
- The desktop interface passed all 176 tests across 49 files, Svelte type
  checking with no warnings, the production build, formatting, localization and
  command-boundary audits.
- All seven bundled weather-provider tests passed.
- The changed Rust files were formatted directly. The repository-wide
  `cargo fmt --all -- --check` remains unable to provide a clean signal on this
  Windows checkout because it reports the existing newline-style mismatch
  across many untouched crates; this audit did not rewrite unrelated files.

## Compatibility and privacy decision

No Sentry event schema, project, DSN, release identity, or public telemetry claim
changes. Local diagnostic JSON lines gain one optional `plugin_id`; old entries
deserialize without it. Privacy Notice version `2026-07-19.3` discloses that
bounded local field and states that it is excluded from Sentry.

## Remaining operational gates

Before a public build embeds DSNs, the maintainer still needs to verify Sentry
project-side IP/data scrubbing, retention, access roles, quota controls, one
sanitized event per runtime, source-map/native-symbol handling, data-processing
terms, and the current subprocessor disclosure. This audit does not satisfy or
bypass those live operational gates.

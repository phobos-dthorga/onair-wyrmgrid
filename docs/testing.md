# Testing strategy

Tests are part of a feature, not follow-up work. A change is complete only when
the relevant automated checks describe its intended behaviour and pass in CI.

## Physical separation

Production modules contain business and application behaviour only. Rust unit
tests live under each crate's `src/tests/` directory and are connected to the
module they exercise by a small `#[cfg(test)]` path hook. Cross-crate and public
contract tests belong in the crate-level `tests/` directory. Svelte and
TypeScript tests use dedicated `*.test.ts` files beside the area they exercise.

This layout keeps private-unit access where it is useful without interleaving
test bodies with production code. Rust removes the test hook and all referenced
test code from normal application builds.

## What each change should test

- Domain rules: valid boundaries, invalid boundaries, malformed values,
  duplicates, capacity limits, and missing optional facts.
- Application services: successful decisions, mismatches, stale or unavailable
  data, retries, concurrency guards, and privacy guarantees.
- Provider adapters: sanitized fixtures, exact request shape, authentication
  boundaries, redirects, timeouts, response-size limits, malformed responses,
  and provider error classification. Live credentials never enter fixtures or
  CI.
- Storage: migrations on empty and previously released databases, encrypted
  open with correct and incorrect keys, portable-backup round trips, corrupt or
  wrong-password restores, retention behaviour, staged activation, and rollback.
- Protocols and community data: versioned fixtures, unknown fields, size limits,
  deny-by-default permissions, and backwards-compatibility decisions.
- Interface: important user journeys, empty/loading/error states, keyboard use,
  and localized text expansion. Business-rule assertions belong in Rust rather
  than Svelte tests.

A bug fix starts with a failing regression test at the lowest meaningful layer.
Avoid tests that merely repeat a type definition or lock in incidental markup.

## Automated gates

Every pull request runs Rust formatting, compilation, Clippy with warnings
denied, core tests, frontend formatting, Svelte type checking, frontend tests,
and a production frontend build. Windows also compiles and tests the Tauri
backend. Dependency review, Rust dependency policy, and high-severity npm audit
checks run in the security workflow. Windows also compiles and tests the
SimConnect provider so its native ABI declarations do not hide behind the
cross-platform unavailable stub.

Launch-art presentation tests cover dark/light theme selection, malformed
colour fallback, and bounded minimum display timing. Every production frontend
build also verifies that both approved hero-image checksums were packaged.

Pull requests produce a downloadable Rust LCOV coverage report. Coverage is a
map for finding untested decisions, not a score to game. A minimum threshold can
be introduced once several releases establish a realistic baseline; until then,
reviewers should reject meaningful coverage regressions in changed business
logic.

## Priority expansion

1. Simulator telemetry contracts: recorded synthetic frames, disconnects,
   reconnects, out-of-order updates, impossible values, rate limits, and safe
   degradation when a bridge or simulator is absent. The version-one fixtures,
   domain boundaries, handshake, replay rejection, raw-value translation,
   orderly shutdown, absent-provider paths, development discovery, and Tauri
   sidecar staging are covered; deterministic reconnect/rate tests and the live
   matrix remain.
2. OnAir synchronization: partial provider failures, rate-limit recovery,
   atomic snapshot publication, and no credential leakage across every error
   path.
3. Dispatch decisions: route, payload, schedule, aircraft, weather freshness,
   and unavailable-evidence branches.
4. Storage evolution: migration tests beginning with the first released schema
   and recovery from individual corrupt snapshots.
5. Desktop journeys: first-run privacy choices, disconnected Atlas, read-only
   Dispatch import, Hoard history selection, language packs, and plugin
   permission review.

## Local AI-agent work

A smaller local agent is well suited to adding table-driven boundary cases,
fixture variants, regression tests for an already-understood defect, and test
documentation. Give it one named behaviour and the command that proves success.
Its changes still require human review and the same CI gates.

Do not delegate interpretation of live provider behaviour, security or privacy
boundaries, protocol compatibility decisions, or assertions that could
silently redefine a business rule. Test-only pull requests should not change
production behaviour merely to make a test pass.

## Live simulator certification

Live tests run outside the repository and CI. Record the WyrmGrid release,
provider version, simulator build, SimConnect client version, architecture, and
aircraft class. Exercise cold start with no simulator, connect, aircraft load,
pause/unpause, on-ground and airborne telemetry, disconnect, simulator exit, and
reconnect. Compare displayed units and a sanitized sample against simulator
values. Record only pass/fail and non-identifying summaries: never retain route,
coordinates, registration, username, local path, raw frame, or provider error.

Passing synthetic tests is not evidence that every aircraft exposes equivalent
facts. Live certification must name the tested scope and keep unsupported
third-party-aircraft variables optional.

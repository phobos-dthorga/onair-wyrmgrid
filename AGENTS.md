# OnAir WyrmGrid contributor instructions

## Product boundaries

- Treat the public OnAir API as read-only unless current official documentation
  explicitly establishes a supported write operation.
- Never log, serialize, expose to plugins, or commit an OnAir API key.
- Raw OnAir JSON belongs in `wyrmgrid-onair-api`; translate it into stable
  WyrmGrid domain models before other modules consume it.
- Preserve the distinction between raw facts, external facts, calculations, and
  recommendations using provenance metadata.
- Community plugins are out-of-process. Do not introduce public Rust, C++, Qt,
  or operating-system ABI coupling.
- Plugin permissions are deny-by-default and capability-oriented.

## Architecture

- Keep UI code presentational. Business rules belong in Rust application or
  domain services, never in Svelte event handlers.
- Keep Tauri commands thin and delegate to `wyrmgrid-application`.
- Keep SQLite migrations append-only after release. Never edit an already
  shipped migration; add a new numbered migration.
- Prefer a few cohesive crates over premature crate fragmentation.
- Reduce duplication and magic strings immediately when the shared abstraction
  is clear, but do not generalize hypothetical requirements.
- Native simulator integrations are separate sidecars and must degrade safely
  when MSFS, SimConnect, or FSUIPC is absent.

## Quality gates

- Rust: formatting, Clippy with warnings denied, unit tests, and dependency audit.
- Frontend: Svelte type checking, production build, and formatting.
- Keep test implementations physically separate from production modules. Rust
  production files may contain only a `#[cfg(test)]` path hook; put unit-test
  bodies in `src/tests/` and black-box integration tests in `tests/`. Keep
  frontend tests in dedicated `*.test.ts` files.
- Every bug fix needs a regression test at the lowest layer that can reproduce
  it. New business rules need boundary, failure, and unavailable-data cases as
  well as the successful path.
- Protocol changes require fixtures, validation tests, documentation, and an
  explicit compatibility decision.
- Security-sensitive changes require corresponding threat-model updates.
- Do not claim live OnAir behavior without a sanitized captured response or an
  authenticated integration test performed outside the repository.

## Releases

- Use semantic versioning for the application and separately version the plugin
  protocol, schema, and database migrations.
- Keep `CHANGELOG.md` as the canonical source for GitHub release notes. Curate
  user- and developer-visible work under `[Unreleased]` using the required New
  features, Changes, Removed, and 🚨 Breaking changes sections; use an
  explicit `- None.` rather than omitting an empty category.
- Release-note and changelog curation may be entirely manual. Optional local-AI
  assistance must use one of the built-in versioned task contracts with a
  bounded, review-only handoff. Change-impact, test-matrix, documentation-sync,
  fixture-variant, bounded implementation-patch, failure-triage, and
  release-curation drafts remain untrusted evidence. Reconcile every draft
  against source, deterministic tools, and tests before using it.
- Hoardmind is the current maintainer's private local assistant, not a WyrmGrid
  component or requirement. Never assume it exists on another contributor's
  machine. WyrmGrid must remain usable, buildable, testable, contributable, and
  releasable without Hoardmind or any other AI.
- Never send credentials, raw provider payloads, personal data, or unrelated
  source context in an AI release-curation packet. Do not run an opaque
  model call on a GitHub release runner; CI validates and publishes the reviewed
  checked-in changelog entry deterministically.
- Run the approved local handoff through
  `scripts/run-optional-ai-task.mjs` with an explicitly selected task and
  versioned profile so exact server-reported tokens and available timing or
  model-allocation metadata are retained outside the repository. Never
  automatically chain model output into another task; a reviewer must select
  confirmed evidence for each new packet. Schema version 1 supports
  unauthenticated loopback Ollama and OpenAI-compatible chat servers; the latter
  must report exact token usage, while non-portable timing, resource, and unload
  fields remain explicitly unavailable. A LAN, authenticated, or hosted adapter
  requires a separate privacy and security decision. Do not relabel local tokens
  as hosted tokens saved unless separate hosted usage data supports that
  calculation.
- A wholly assistant-generated textual patch may be published only through the
  hash-bound `scripts/optional-ai-contribution.mjs` workflow with explicit
  one-invocation approval and a dedicated least-privileged GitHub App. The
  assistant never receives the App private key or installation token. Generated
  changes use `assistant/<assistant-id>/<contribution-id>` and one bot commit;
  after the Contents-only App token is discarded, the human maintainer opens the
  draft PR. Generated changes cannot target protected policy, dependency,
  migration, legal, security, protocol/schema, release, workflow, or optional-
  AI governance paths. `main` must require pull requests without an App bypass.
  The App has no Pull requests, review, merge, version, tag, release, Actions,
  workflow, administration, secret, or organization authority. A person must
  run the normal local gates and explicitly decide whether to land the PR.
  Human-written or materially rewritten changes remain human-authored and use
  `Assisted-by:` when attribution is useful.
- Land an assistant-generated PR only through the human-authenticated
  `scripts/optional-ai-landing.mjs` guard. It must bind the reviewed manifest
  hash, exact one-commit head, App-bot identity, clean protected PR state, and
  one-invocation approval; provide an explicit provenance-preserving squash
  subject and body; forbid administrative bypass; and verify the resulting
  merge commit. Do not use a default GitHub squash message for a generated
  contribution.
- A declared application breaking change requires a new `X.0.0` major release
  line and must remain prominently identified in the changelog and generated
  GitHub notes. Minor and patch release tags containing a breaking-change entry
  must fail release policy.
- CI produces release artifacts. Do not hand-assemble published binaries.
- Run routine compilation, tests, formatting, linting, and dependency checks on
  the maintainer's local development machine. Reserve hosted CI/CD for an
  explicitly authorised release or exception. Every intentional semantic-version
  release tag (`vX.Y.Z` or a supported prerelease) repeats the complete gates on
  clean hosted runners and builds installers. A manual rebuild must target an
  existing tag and record a concrete reason.
- Existing pull-request workflow triggers may still run until they are changed
  separately. Do not treat those transitional triggers as authority to request
  reruns, wait for hosted results, or spend hosted minutes during routine work.
- Keep early releases marked as prereleases until update signing and platform
  signing policies are complete.
- Do not change the application semantic version or create or push a release
  tag without explicit maintainer authorization. Internal schema, protocol,
  migration, legal-document, and catalogue compatibility markers still advance
  with the change that requires them and do not trigger an installer build.
- Preserve the Windows installer's product name, application identifier, and
  per-user scope. Any intentional identity change requires an explicit migration
  design; routine setup upgrades must preserve application data and its
  device-local encryption key.

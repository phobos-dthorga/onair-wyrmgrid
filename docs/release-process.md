# Release process

1. Keep user- and developer-visible work curated under `[Unreleased]` in the
   root `CHANGELOG.md` throughout development. Every entry retains explicit New
   features, Changes, Removed, and 🚨 Breaking changes sections.
2. After the maintainer authorizes a version, curate the release manually or
   optionally give a user-selected local assistant a bounded, review-only
   packet containing the prior release tag, candidate commit range, file-level
   change summary, and current `[Unreleased]` text. Do not include credentials,
   raw provider payloads, personal data, or unrelated source context.
3. Reconcile the draft against the actual diff, tests, compatibility decisions,
   and documentation. Move the reviewed `[Unreleased]` content into a dated
   `[X.Y.Z]` entry and create a fresh empty `[Unreleased]` entry. AI output, when
   used, is untrusted assistance rather than release authority.
4. Update the application version in the root and desktop `package.json` files,
   `Cargo.toml`, and `apps/desktop/src-tauri/tauri.conf.json`. Update extended
   compatibility, protocol, database, or migration documentation when needed;
   `CHANGELOG.md` remains the canonical GitHub release-note source.
5. Run `npm run test:tooling`,
   `node scripts/verify-release-version.mjs <version>`, and
   `node scripts/verify-installer-contract.mjs`. Preview and validate the exact
   GitHub text with
   `node scripts/prepare-release-notes.mjs <version> --previous <version>`, then
   complete the normal local checks on the maintainer's development machine.
6. Create and push an annotated `vX.Y.Z` tag from a commit on `main`. Supported
   prerelease suffixes such as `v0.2.0-rc.1` are also accepted; build metadata is
   deliberately excluded from installer versions.
7. The release workflow reuses the complete CI and security workflows against
   that exact tag. Packaging cannot begin until every required job passes.
8. GitHub-hosted runners build the Windows NSIS setup executable, Linux AppImage
   and Debian package, and macOS DMG. The Windows runner silently installs the
   NSIS output and verifies that both the desktop executable and SimConnect
   provider sidecar are present. After the first release, it installs the
   closest older published setup first, installs the new setup over it, and
   verifies that existing application data survives.
9. A single least-privilege publication job downloads the platform outputs,
   produces `SHA256SUMS.txt`, records GitHub artifact provenance, extracts the
   matching reviewed changelog entry, and attaches the files and notes to a
   draft prerelease.
10. Verify artifact installation, startup, licence notices, checksums, release
    notes, and basic
    offline behaviour on every supported platform. Publish the GitHub release
    only after manual verification.

Early releases stay prereleases. Platform signing and updater signing must be
documented and tested before automatic updates or stable releases are enabled.

## Changelog and breaking-change policy

The checked-in changelog is the single editorial source for release summaries.
The maintainer or release agent checks every claim against repository evidence
before the entry can reach `main`, whether the first draft was written manually
or with optional local assistance. GitHub Actions does not ask a model to infer
release content from commits, so a tagged build remains reproducible,
reviewable, and free of model credentials or inference costs.

Each release entry must list:

- new user- or developer-facing features;
- changed behaviour, architecture, or operational requirements;
- removed capabilities, including an explicit `- None.` when applicable; and
- 🚨 breaking changes, again with an explicit `- None.` when applicable.

If the breaking-change list is not empty, release policy requires a new
`X.0.0` major line. The same prominent warning may continue through prereleases
of that exact major version. A minor or patch release that declares a breaking
change fails before packaging, as does a tag without a complete matching
changelog entry. The generated GitHub body adds a caution banner above an
actual breaking-change release and an explicit no-breaking-change notice for
all others.

## Optional local-AI curation and efficiency capture

This step is entirely optional. WyrmGrid users and contributors do not need an
AI assistant, local model server, profile, or this runner. Hoardmind is the
maintainer's private local configuration and is neither invoked nor named by the
generic tool. Any user may copy an example profile, choose their own local model
and boundary prompt, and keep that private configuration under
`.wyrmgrid-local/`.

Copy the
[release-curation handoff template](optional-ai/templates/release-curation-v1.md)
to a temporary Markdown file outside the repository and complete only its
bounded fields. It should contain only the previous tag, candidate commit,
bounded commit and file summaries, compatibility decisions, and current
`[Unreleased]` text needed for the curation pass. Copy and adapt either the
[Ollama profile](../examples/optional-ai/local-ollama-profile-v1.json) or
[OpenAI-compatible profile](../examples/optional-ai/openai-compatible-local-profile-v1.json).
When the copy lives in `.wyrmgrid-local/`, set its
`system_prompt_file` to
`../docs/optional-ai/base-system-prompt-v1.md`; profile-relative paths
make the private file portable without embedding a maintainer-specific absolute
path. Then run:

```powershell
$profilePath = '.wyrmgrid-local\release-curation-profile.json'
$reportRoot = Join-Path $env:TEMP 'wyrmgrid-optional-ai-release'

npm run ai:release-curation -- `
  --packet (Join-Path $env:TEMP 'wyrmgrid-release-handoff.md') `
  --profile $profilePath `
  --output $reportRoot `
  --approve-once
```

The version-1 [profile schema](../schemas/optional-ai-task-profile-v1.schema.json)
lets the user select a model, safe boundary prompt, output limit, temperature,
and deterministic seed. Ollama profiles additionally select context size and
thinking mode. Both adapters accept only an unauthenticated HTTP origin on
`127.0.0.1`, `localhost`, or `::1`; LAN, authenticated, and hosted providers
need a separate privacy and security design before support is added. The runner
refuses CI, requires one-invocation approval, rejects common credential
signatures and packets over 64 KiB, validates the selected boundary prompt, and
rejects silent model substitution.

The adapters are deliberately small:

- `ollama-chat` uses Ollama's native chat, version, and loaded-model endpoints.
  It requests zero keep-alive, samples model allocation while the request runs,
  and verifies that the model unloads afterward.
- `openai-compatible-chat` uses unauthenticated `GET /v1/models` and
  non-streaming `POST /v1/chat/completions`. It sends no tools or authorization
  header, verifies the selected model before and after inference, and requires
  exact, internally consistent `usage` token counts. This covers the local APIs
  documented by [LM Studio](https://lmstudio.ai/docs/developer/core/server),
  [LocalAI](https://localai.io/basics/getting_started/index.html), and
  [llama.cpp](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).

The broader [optional local-AI task guide](optional-ai/README.md) documents the
other bounded development tasks. Release curation remains its own versioned
contract and never consumes another task's draft without explicit review.
Release content, compatibility, semantic versioning, tags, and publication are
critical boundaries under the local-AI review budget, so every release-curation
draft still receives full reconciliation regardless of its apparent size.

Each run writes three timestamped local artifacts:

- `*-draft.md` contains the optional assistant's draft for human reconciliation;
- `*-metrics.json` contains schema-version 1 structured measurements without
  the packet, system prompt, or response content; and
- `*-efficiency.md` presents the measurements in a release-ready table.

Both adapters retain exact server-reported prompt, generated, and total token
counts plus client-observed request duration. Ollama also supplies model-load,
prompt-evaluation, generation, throughput, version, and sampled RAM/VRAM data.
Some OpenAI-compatible servers, notably llama.cpp, add non-standard timing
fields which the report captures when present. The compatibility standard does
not portably expose runtime version, RAM/VRAM allocation, or model-unload
control, so the report marks those measurements `Not reported` or `Not
observable` instead of presenting zeros. Advertised model-file, training-context,
and parameter metadata from `/v1/models` is retained separately when supplied.
A sampling error is counted explicitly rather than converted into zero-cost
success.

These measurements show work performed locally. They do not by themselves
prove how many OpenAI or other hosted tokens were avoided, because a
coordinating agent may still prepare or review the packet. Report hosted token
or monetary savings only when separate hosted usage data provides a defensible
comparison. Keep the temporary packet and generated draft private, review the
draft against repository evidence, and remove local temporary artifacts when
they are no longer useful.

## Installing a newer setup

The supported manual update path is to close WyrmGrid and run the newer NSIS
setup. Users do not need to uninstall the previous version. The installer keeps
the stable `OnAir WyrmGrid` product name, application identifier
`io.github.phobosdthorga.onairwyrmgrid`, and per-user installation scope so it
recognises and replaces the existing application. Downgrades are refused.

The setup replaces application binaries and bundled sidecars only. The encrypted
database remains under the user's roaming application-data directory and its
device-local key remains in the operating-system credential store. Neither is
embedded in, regenerated by, or removed by a normal setup upgrade. Append-only
database migrations update stored data when a newer application needs them.

Changing the product name, application identifier, or installation scope would
break that continuity and therefore requires a separately designed data and
installer migration. The installer-contract tooling intentionally rejects such
drift. A portable backup is still recommended before prerelease upgrades.

## Installer and hosted-runner policy

Routine commits and pull requests compile-check and test the desktop locally;
hosted CI/CD is reserved for releases unless the maintainer explicitly approves
an exception. Every intentional semantic-version release tag repeats the full
validation suite on clean hosted runners and produces packages, including patch
releases. This makes the tag both an unambiguous request for a complete release
candidate and the point where independent hosted verification has material
value.

Release pull requests named `codex/release-*` run the branch-protection checks
needed before the approved version can reach `main`. Other pull requests report
those jobs as skipped and do not allocate hosted runners. The complete CI and
security workflows run again against the immutable release tag, while the
security workflow additionally retains its scheduled dependency review. Avoid
unnecessary manual dispatches and reruns; local gates remain authoritative for
ordinary development.

The release policy rejects malformed versions, tags outside `main`, and any tag
whose version differs from the four checked-in application version sources. A
manual dispatch is a rebuild of an existing tag, never a way to package mutable
branch contents under a release name, and requires a meaningful recorded reason.
The resulting packages remain CI outputs; local builds are verification only and
are never hand-published.

## Key and secret boundaries

End-user data-protection material is never a build secret. Each installation
generates its SQLCipher device key locally and stores it in that user's operating-
system credential service. Remembered provider credentials follow the same
device-local boundary. Embedding either value in CI would give installations a
shared extractable secret and is prohibited.

CI may eventually hold narrowly scoped release credentials that authenticate an
artifact rather than decrypt user data:

- Windows code-signing credentials;
- the Tauri updater private signing key and its password; and
- platform notarisation credentials.

Those credentials must live in a protected GitHub release environment, be
available only to the exact signing job, and have a separately protected recovery
copy where loss would prevent future updates. Public verification keys may be
compiled into WyrmGrid. Pull-request jobs, community code, application runtime,
plugins, installers, logs, and support bundles never receive private release
credentials.

CI tests credential and database behaviour with disposable fakes or ephemeral
keys generated for that job. Test keys and runner files are destroyed with the
runner and are never valid for an end-user installation.

## Supply-chain controls

- Workflow dependencies are pinned to immutable commit hashes. Dependabot keeps
  GitHub Actions updates visible for review.
- Build jobs have read-only repository access and cannot create or alter a
  release. Only the final attestation/publication job receives `contents: write`.
- Release tags identify commits on `main`, and every platform builds the same
  checked-in tag.
- SHA-256 checksums detect accidental corruption; GitHub artifact attestations
  associate packages with the repository, workflow, commit, and triggering tag.
- The draft release remains a human promotion boundary while releases are
  unsigned prereleases.

Branch protection should restrict `main` updates to the maintainer and keep
force pushes and branch deletion disabled. The immutable release tag then runs
Rust core, Frontend, Windows desktop, `rust-audit`, and `npm-audit` checks before
packaging begins.

## Diagnostic artifacts

When Sentry integration is enabled for a release:

- Rust and SvelteKit events use the same canonical release identifier,
  `onair-wyrmgrid@<semver>`, with platform or channel represented separately.
- Browser source maps and native debug information are generated and uploaded by
  the release workflow before packaging or stripping. Source maps are not shipped
  in the public application bundle solely for Sentry's benefit.
- `SENTRY_AUTH_TOKEN` and equivalent organisation or project credentials exist
  only as protected CI secrets. Routine pull-request builds do not receive them
  and do not create Sentry releases.
- A failed diagnostic-artifact upload blocks the platform build and therefore
  prevents draft publication. It does not justify rebuilding binaries by hand.
- Before the first stable release on a supported platform, a sanitised synthetic
  failure must demonstrate symbolicated Rust and Svelte stack traces against the
  exact CI-built artifacts.

See the [observability plan](operations/observability.md) for the phased rollout
and [ADR-0007](architecture/decisions/0007-hosted-sentry-observability.md) for the
hosting and privacy decision.

## Sidecars and provider assets

When a release includes WyrmGrid Bridge or another provider adapter:

- version the application, Bridge protocol, plugin protocol, database
  migrations, and external fixture/schema compatibility independently;
- build each platform sidecar in CI from the same tagged commit, sign it under
  the platform policy when signing is enabled, record its checksum, and package
  it through Tauri rather than copying a local executable into an installer;
- run the checked-in provider preparation command so Tauri receives the
  target-triple-suffixed release executable declared in
  `tauri.windows.conf.json`; never reuse the ignored local staging file as a
  release input. Non-Windows bundles do not declare this provider;
- include only sidecars supported on that target platform and verify that the
  desktop starts, reports a clear unavailable state, and exits cleanly when no
  simulator is installed;
- smoke-test protocol handshake, version mismatch, supervised shutdown, and
  tampered-sidecar rejection before promotion;
- review redistribution terms for SimConnect, FSUIPC, simulator SDK, reference
  data, and every bundled native dependency before packaging; and
- never bundle provider application secrets, user tokens, private OFPs,
  SayIntentions keys or `flight.json` captures, downloaded Navigraph packages,
  live network captures, or authenticated test data in release artifacts.

Adding or changing a Bridge message or provider snapshot requires fixtures,
validation tests, documentation, and an explicit compatibility decision. A
working local simulator test is not a substitute for those release artifacts.

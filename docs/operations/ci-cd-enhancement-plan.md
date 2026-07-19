# CI/CD hardening and enrichment plan

**Status:** Proposal only

**Reviewed baseline:** 19 July 2026
**Authority:** This document records candidate work. It does not authorize a
workflow run, GitHub setting change, cache deletion, version change, tag,
release rebuild, publication, signing operation, or optional-AI contribution.

## Purpose

WyrmGrid already has a deliberately local-first development process and a
careful tagged-release pipeline. This plan records the improvements identified
by a read-only audit so they can be discussed, implemented in small stages, and
verified without quietly expanding GitHub or optional-AI authority.

In this document, CI/CD means automated checking, packaging, and release
delivery. The currently implemented process remains defined by the
[testing strategy](../testing.md), [release process](../release-process.md), and
[optional local-AI policy](../optional-ai/README.md). If this proposal differs
from those documents, the implemented-process documents remain authoritative
until an approved change updates them together.

## Existing strengths to preserve

- Routine compilation, testing, formatting, linting, and dependency checks run
  on the maintainer's local development machine.
- Hosted GitHub Actions are reserved for release pull requests, release tags,
  scheduled security checks, and explicitly authorized exceptions.
- Workflow jobs start with read-only repository permissions. Only the final
  release-publication job receives release-writing authority.
- Workflow actions are already referenced by full commit hashes and Dependabot
  proposes updates to them.
- Release tags must identify a commit on `main`, and application version files
  and curated changelog sections must agree with the tag.
- Release packages receive SHA-256 checksums and GitHub build attestations.
- Releases begin as draft prereleases and require manual verification before
  publication.
- Windows release validation covers a clean NSIS install, bundled SimConnect
  provider presence, and an in-place upgrade that preserves application data.
- Secret scanning, push protection, and Dependabot security updates are
  enabled.
- Hoardmind is local and optional. Its generated-contribution GitHub App has no
  pull-request, review, merge, workflow, secret, tag, release, or administration
  authority.

These controls should not be weakened merely to simplify implementation.

## Confirmed gaps and proposed responses

| Area                     | Confirmed present state                                                                                                                                   | Proposed response                                                                                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release identity         | Release policy resolves an exact commit SHA, but reusable checks and platform builds receive the version tag again.                                       | Pass the resolved commit SHA to every checkout, test, security, build, and publication step.                                                                           |
| Tag protection           | Version tags are not covered by a tag ruleset before publication.                                                                                         | Add a `v*` tag ruleset that prevents updates and deletion after creation.                                                                                              |
| Published assets         | A manual rebuild may replace assets for any existing release with the selected tag.                                                                       | Permit replacement only while the release is a draft. A defective published release requires a new version.                                                            |
| Pull-request checks      | Ordinary pull-request jobs are skipped. GitHub records skipped required jobs as successful, so green checks do not prove hosted tests ran.                | Add one small pull-request policy check and one required validation summary that clearly distinguishes approved local-only validation from required hosted validation. |
| Release coverage         | The v0.2.0 release run skipped the promised Rust LCOV coverage job.                                                                                       | Use an explicit reusable-workflow input to request coverage and keep the coverage report separate from installer assets.                                               |
| Boundary audits          | Deterministic localization and desktop-command audits exist but are not part of hosted release validation.                                                | Add `npm run audit:boundaries` to the complete release gate.                                                                                                           |
| Rust release gates       | Core Rust receives strict Clippy checks, but the Windows desktop and Windows-specific provider do not receive an equivalent hosted lint gate.             | Add Windows Clippy with warnings denied and require locked Cargo dependency resolution.                                                                                |
| Database evolution       | Migration behaviour is tested, but CI does not mechanically prove that previously released migration files were not edited, deleted, or renumbered.       | Compare with the prior release and permit only new, contiguous, append-only migrations.                                                                                |
| Package completeness     | Publication gathers produced files without an authoritative expected package and architecture manifest.                                                   | Validate exact package counts, names, architectures, and required bundled sidecars before publication.                                                                 |
| Workflow policy          | Workflow source uses pinned actions, but repository settings allow all actions and do not require full-SHA pinning.                                       | Allow only reviewed action repositories and enable GitHub's SHA-pinning requirement.                                                                                   |
| Protection rules         | Classic branch protection and a repository ruleset overlap and disagree on some settings; the ruleset's release pattern does not match `codex/release-*`. | Consolidate on one reviewed protection design with no unintended bypass actor.                                                                                         |
| Build caches             | Actions caches total approximately 11.2 GB across 28 entries, above GitHub's default 10 GB allowance.                                                     | Prefer dependency-download caches over compiled `target` caches, stop producing low-value ref-scoped caches, and review exact stale entries before any deletion.       |
| Release secrets          | There are no GitHub deployment environments; the Sentry upload credential is repository-scoped.                                                           | Isolate observability upload and publication authority in narrowly scoped protected environments.                                                                      |
| Publication              | Draft publication is automated, but final promotion is an informal manual action.                                                                         | Add a separately approved promotion workflow that verifies the reviewed draft before publishing it immutably.                                                          |
| Supply-chain inventory   | Packages are attested, but an exact-release software bill of materials is not attached and attested.                                                      | Generate an SPDX software bill of materials from the exact release commit and attest it with the packages.                                                             |
| Source security analysis | Dependency audits run, but repository code scanning has no completed analysis.                                                                            | Add scheduled and release-only CodeQL analysis for supported Rust and JavaScript/TypeScript source.                                                                    |
| Platform scope           | The published macOS package is Apple Silicon-only while general documentation says macOS.                                                                 | Make a separate supported-architecture decision: Apple Silicon-only, separate Intel and Apple Silicon packages, or a universal package.                                |

An attestation is signed provenance evidence connecting an artifact to its
source commit and workflow. It does not prove that the software is free of bugs
or vulnerabilities. A software bill of materials is an ingredient list, not a
security verdict.

## Target process

The intended end state is:

1. Development and complete routine validation happen locally.
2. Optional Hoardmind review receives only a selected, sanitized, bounded
   packet and never determines whether a gate passed.
3. A pull request runs a small policy check.
4. The policy check classifies the change using deterministic paths, trusted
   maintainer labels, and event facts.
5. Ordinary work records that local validation is the approved requirement.
6. Release, dependency, migration, protocol, schema, security, installer,
   workflow, optional-AI governance, or explicitly approved exception work
   receives the relevant hosted checks.
7. One required validation-summary job succeeds only when every check required
   by that classification succeeded.
8. An authorized release pull request prepares the version and curated
   changelog, then receives the complete hosted gates.
9. A protected version tag starts the release and resolves once to an exact
   commit SHA.
10. Every check and platform package uses that exact commit.
11. CI validates package contents, generates checksums, build metadata, a
    software bill of materials, and attestations, then creates or updates only
    a draft prerelease.
12. The maintainer performs the documented real-platform checks.
13. A protected promotion workflow verifies the reviewed evidence and
    publishes an immutable prerelease.

## Staged implementation

Each stage requires a fresh worktree, branch, open-pull-request, and task
inventory before edits begin. Each stage should remain independently
reviewable and revertible.

### Stage 1 — release integrity and coverage correctness

Proposed repository work:

- use the resolved release commit SHA everywhere;
- refuse asset replacement for a published release;
- repair the explicit release-coverage request;
- download only named platform package artifacts for publication;
- add sensible workflow timeouts and disable persisted checkout credentials
  where they are unnecessary;
- add regression tests for all of those controls; and
- synchronize the implemented release and testing documentation only after the
  behaviour exists.

Acceptance evidence:

- repository tooling tests cover exact-SHA propagation, draft-only replacement,
  coverage selection, and artifact filtering;
- workflow formatting and static validation pass;
- existing repository-tooling tests pass; and
- the diff contains no application version change, new tag, release mutation,
  GitHub setting mutation, or unrelated application feature change.

Stage 1 should be completed locally and reviewed before any push or hosted run.

### Stage 2 — truthful pull-request validation

Proposed policy change:

- run one inexpensive pull-request policy check for every pull request;
- determine whether hosted jobs are required without trusting a contributor's
  branch name alone;
- treat release, dependency, migration, protocol, schema, security, privacy,
  legal, credential, authorization, cryptography, installer, workflow,
  signing, optional-AI governance, and other uncertain critical changes as
  requiring hosted validation;
- allow a trusted maintainer label to request broader validation, never less;
  and
- require a final `Required validation` summary that examines every conditional
  job result even when a dependency fails or is skipped.

This stage creates a small permanent hosted-runner exception for the policy
check. The maintainer must explicitly approve that policy and expected runner
use before implementation.

Acceptance evidence:

- fixtures cover ordinary, release, dependency, generated-contribution, and
  every protected-path classification;
- an untrusted branch name or pull-request text cannot downgrade validation;
- required failures and unexpected skips block the summary; and
- ordinary work receives an honest explanation rather than a false claim that
  hosted tests ran.

### Stage 3 — complete deterministic release gates

Proposed repository work:

- add frontend identifier-boundary audits;
- add strict Windows desktop and provider Clippy checks;
- enforce locked Rust dependency resolution in checks and packages;
- enforce append-only released migrations and contiguous new migration numbers;
- validate expected package names, counts, architectures, and sidecars;
- create a privacy-safe `BUILD-METADATA.json` with the tag, exact commit,
  workflow run, runner images, rebuild reason when applicable, and artifact
  digests; and
- reduce future cache growth without deleting an unreviewed cache target.

Protocol and schema changes still require fixtures, validation tests,
documentation, and an explicit compatibility decision. A deterministic path
gate can require that evidence to be present, but it cannot make the
compatibility decision.

### Stage 4 — GitHub settings and controlled promotion

Proposed external GitHub changes:

- add a version-tag ruleset;
- consolidate overlapping `main` and release-branch protections;
- require full-SHA action references and allow only reviewed action sources;
- create narrowly scoped release-observability and release-publication
  environments;
- move a release credential only after the consuming job references the correct
  protected environment;
- enable immutable releases; and
- add a separate promotion workflow that verifies the draft's checksums,
  attestations, exact commit, manual-review reference, and prerelease status.

Before applying any setting, the implementation task must display the current
setting, proposed setting, effective branches or tags, bypass actors, and a
rollback path. Repository files should land before a setting that depends on
them is enabled. The existing v0.2.0 draft is not altered, rebuilt, published,
or deleted by this plan.

### Stage 5 — supply-chain and platform enrichment

Proposed work:

- add scheduled and release-only CodeQL analysis;
- create and attest an exact-release SPDX software bill of materials;
- document how users verify checksums, build attestations, and immutable
  releases;
- choose and document the macOS architecture matrix;
- use deliberately selected runner-image versions and record their identities;
  and
- add safe Linux and macOS package-structure checks while retaining real
  operating-system startup checks as a manual prerelease boundary where
  automation is not trustworthy.

### Stage 6 — optional Hoardmind conveniences

The detailed candidate architecture, safety boundaries, implementation stages,
and validation plan are recorded in the
[local review automation and bounded Hoardmind delegation plan](local-review-automation.md).
This section remains the CI/CD programme boundary for those proposed helpers.

The deterministic Stage 1 inventory is now implemented as
`npm run review:inventory`. It records a versioned local source-evidence bundle
without running validation, preparing an AI packet, invoking a model, reusing a
cache, or changing CI/CD. The remaining helpers below are still proposals.

Proposed local-only helpers:

- prepare a bounded change-impact, test-matrix, documentation-sync, or fixture
  packet from maintainer-selected source evidence;
- prepare a sanitized failure-triage packet from one explicitly selected local
  or GitHub failure;
- assemble deterministic release-readiness evidence before the existing
  release-curation task; and
- consider a separately versioned generated-contribution receipt that binds the
  exact base commit, deterministic critical-path classification, required local
  gates, and a digest of reviewed results.

The helper may reduce packet-preparation effort. It must not automatically run
a model, feed one model response into another task, upload private artifacts,
or convert a model's claim into test, compatibility, release, or approval
evidence.

Hoardmind remains:

- optional and replaceable;
- local and loopback-only under the approved version-1 adapters;
- no-tools and review-only;
- unable to receive GitHub, OnAir, Sentry, signing, or other credentials;
- unable to create or change versions, tags, releases, rules, environments, or
  secrets;
- unable to approve, merge, promote, sign, or publish; and
- subject to mandatory semantic review for every CI/CD, release, security,
  migration, protocol, schema, signing, installer, or optional-AI governance
  result.

The generated-contribution GitHub App permissions must not be broadened to
implement this stage.

## Coordination and interference controls

Before every implementation stage:

1. record the current branch, worktree changes, open pull requests, and active
   tasks affecting the proposed paths;
2. list the exact repository files and GitHub settings the stage expects to
   touch;
3. stop and inventory overlaps with another task before editing them;
4. avoid broad formatters or mechanical rewrites while unrelated work is
   present;
5. never stage, revert, move, or incorporate another task's changes; and
6. repeat the inventory immediately before a push or external GitHub mutation.

At the time this plan was documented, unrelated in-progress work existed in
plugin persistence, data protection, localization, application and storage
tests, and new migration `0016`. This documentation task did not modify,
format, stage, test, or reinterpret that work. A new inventory is required
because this note will become stale.

Likely overlap areas by stage include:

- Stages 1–3: `.github/workflows/`, release and audit scripts, tooling tests,
  `docs/release-process.md`, and `docs/testing.md`;
- Stage 3: migration policy and tests, without editing any shipped migration;
- Stage 4: repository rulesets, branch protection, Actions permissions,
  environments, secrets, and release settings; and
- Stage 6: `docs/optional-ai/`, optional-AI schemas, examples, and broker or
  landing scripts if a new receipt version is separately approved.

## Intentionally deferred or separately governed work

This plan does not implement or declare readiness for:

- Windows code signing;
- macOS signing or notarization;
- Tauri updater signing or automatic updates;
- stable releases;
- native Sentry PDB, dSYM, or ELF debug-information upload;
- public telemetry activation or embedded runtime DSNs;
- live OnAir behaviour or authenticated integration tests;
- live simulator certification;
- final Intel, Apple Silicon, or universal macOS support;
- redistribution approval for every native simulator dependency;
- an authenticated, LAN, or hosted optional-AI adapter; or
- a self-hosted GitHub runner on the maintainer's development workstation.

Each item needs its own evidence, privacy and security review, credentials or
hardware decision, and explicit maintainer authorization.

## Completion conditions

The plan is complete only when implemented behaviour and GitHub settings are
both documented and verified. For each stage:

- every confirmed bug has a regression test at the lowest useful layer;
- critical path and unavailable-data cases are covered;
- repository tooling, formatting, Rust, frontend, dependency, installer, and
  workflow checks appropriate to the stage pass;
- current GitHub settings are read back after any approved mutation;
- no release is published solely because automation or Hoardmind recommends it;
- the changelog describes implemented behaviour without presenting a proposal
  as complete; and
- the maintainer explicitly decides whether to proceed to the next stage.

## Reference material

- [GitHub: skipped jobs and required status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks)
- [GitHub: immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub: repository Actions permissions and SHA pinning](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository)
- [GitHub: dependency-cache limits and eviction](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- [GitHub: deployment environments](https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments)
- [GitHub: artifact attestations and software bills of materials](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- [GitHub: CodeQL default setup](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configure-code-scanning)

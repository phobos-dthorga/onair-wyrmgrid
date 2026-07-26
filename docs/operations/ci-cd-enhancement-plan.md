# CI/CD hardening and enrichment plan

**Status:** Active follow-up plan; Jenkins foundation implemented

**Reviewed baseline:** 25 July 2026
**Authority:** This document records candidate work. It does not authorize a
workflow run, GitHub setting change, cache deletion, version change, tag,
release rebuild, publication, signing operation, or optional-AI contribution.

## Purpose

WyrmGrid has a local-first development process, publication-credential-free
Jenkins validation with bounded ForgeAI advice, and a separately protected
exact-tag release pipeline. This plan
records remaining improvements identified by audit so they can be discussed,
implemented in small stages, and verified without quietly expanding Jenkins,
GitHub, or optional-AI authority.

In this document, CI/CD means automated checking, packaging, and release
delivery. The currently implemented process remains defined by the
[testing strategy](../testing.md), [release process](../release-process.md), and
[optional local-AI policy](../optional-ai/README.md). If this proposal differs
from those documents, the implemented-process documents remain authoritative
until an approved change updates them together.

## Existing strengths to preserve

- Routine compilation, testing, formatting, linting, and dependency checks run
  locally and in the publication-credential-free Jenkins Organization Folder
  on Linux and Windows. ForgeAI receives only a bounded change packet and
  screened pipeline/dependency inputs after those deterministic stages and has
  no gate or release authority.
- Jenkins snapshots are limited to `main` and `codex/release-*`; ordinary
  branches receive no distributable package.
- Hosted GitHub Actions run only as a manually authorized Windows/Linux release
  fallback for an existing tag and documented reason.
- The trusted Jenkins release job is outside the Organization Folder. Only its
  bounded GitHub CLI commands receive the repository-specific release App token.
- Workflow actions are already referenced by full commit hashes and Dependabot
  proposes updates to them.
- Release tags must identify a commit on `main`, and application version files
  and curated changelog sections must agree with the tag.
- Jenkins release packages receive SHA-256 checksums and explicitly
  non-attested build metadata. The manual GitHub fallback retains native GitHub
  attestations.
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

| Area                     | Confirmed present state                                                                                                                                   | Proposed response                                                                                                                                                |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release identity         | Jenkins passes one exact tag commit to Linux and Windows validation, packaging, assembly, and publication.                                                | Keep regression coverage for exact-commit propagation and refuse mutable branch release inputs.                                                                  |
| Tag protection           | Version tags are not covered by a tag ruleset before publication.                                                                                         | Add a `v*` tag ruleset that prevents updates and deletion after creation.                                                                                        |
| Published assets         | Jenkins refuses to replace a published release and can update only a draft after a fresh exact-tag build.                                                 | Keep the publication-stage recheck so state changes during a build fail closed.                                                                                  |
| Pull-request checks      | Jenkins runs complete Linux and Windows validation for every discovered revision without relying on skipped GitHub checks.                                | Update repository branch protection after both Jenkins check names are observed and stable.                                                                      |
| Release coverage         | The manual GitHub fallback retains LCOV; the initial Jenkins release pipeline does not publish coverage.                                                  | Add Jenkins-native coverage only after retention, presentation, and runtime cost are reviewed.                                                                   |
| Boundary audits          | `npm run audit:boundaries` is part of the Linux Jenkins and manual GitHub fallback gates.                                                                 | Extend deterministic path policy when new identifier boundaries are introduced.                                                                                  |
| Rust release gates       | Jenkins runs locked full-workspace Clippy with warnings denied on Linux and Windows; the fallback adds strict Windows desktop/provider Clippy.            | Keep platform-specific regression coverage as new native crates are added.                                                                                       |
| Database evolution       | Migration behaviour is tested, but CI does not mechanically prove that previously released migration files were not edited, deleted, or renumbered.       | Compare with the prior release and permit only new, contiguous, append-only migrations.                                                                          |
| Package completeness     | Jenkins validates one AppImage, one Debian package, and one NSIS setup before assembly.                                                                   | Add deeper architecture and bundled-sidecar inspection without weakening current exact-count checks.                                                             |
| Workflow policy          | Workflow source uses pinned actions, but repository settings allow all actions and do not require full-SHA pinning.                                       | Allow only reviewed action repositories and enable GitHub's SHA-pinning requirement.                                                                             |
| Protection rules         | Classic branch protection and a repository ruleset overlap and disagree on some settings; the ruleset's release pattern does not match `codex/release-*`. | Consolidate on one reviewed protection design with no unintended bypass actor.                                                                                   |
| Build caches             | Actions caches total approximately 11.2 GB across 28 entries, above GitHub's default 10 GB allowance.                                                     | Prefer dependency-download caches over compiled `target` caches, stop producing low-value ref-scoped caches, and review exact stale entries before any deletion. |
| Release secrets          | Publication uses a release-folder-scoped GitHub App; source-map upload remains disabled in the initial Jenkins release path.                              | Design a separately scoped observability credential before enabling exact-artifact source-map upload.                                                            |
| Publication              | Draft publication is automated, but final promotion is an informal manual action.                                                                         | Add a separately approved promotion workflow that verifies the reviewed draft before publishing it immutably.                                                    |
| Supply-chain inventory   | Jenkins packages have checksums and build metadata but no cryptographic provenance or exact-release software bill of materials.                           | Design Sigstore or equivalent Jenkins identity, generate SPDX, and document consumer verification before claiming attestation.                                   |
| Source security analysis | Dependency audits run, but repository code scanning has no completed analysis.                                                                            | Add scheduled and release-only CodeQL analysis for supported Rust and JavaScript/TypeScript source.                                                              |
| Platform scope           | Official desktop release support is Windows x86-64 and Linux x86-64; macOS protocol values remain for compatibility.                                      | Treat any future macOS desktop release as a new support, signing, packaging, and real-device validation decision.                                                |

An attestation is signed provenance evidence connecting an artifact to its
source commit and workflow. It does not prove that the software is free of bugs
or vulnerabilities. A software bill of materials is an ingredient list, not a
security verdict.

## Target process

The intended end state is:

1. Development validation happens locally and Jenkins repeats the complete
   Linux and Windows gates for every discovered revision.
2. Optional Hoardmind review receives only a selected, sanitized, bounded
   packet and never determines whether a gate passed.
3. Pull requests receive publication-credential-free deterministic Jenkins
   checks; branch protection requires their stable Linux and Windows results.
   Origin pull-request merge builds and `main` additionally receive
   non-blocking ForgeAI advice after deterministic work completes.
4. `main` and `codex/release-*` additionally retain unsigned snapshots.
5. An authorized release change prepares the versions and curated changelog.
6. A maintainer creates an immutable version tag on `main`.
7. The separately protected Jenkins release job accepts the existing tag and a
   meaningful reason, resolves one exact commit, and repeats every gate.
8. Linux and Windows packages use that exact commit.
9. Jenkins validates package counts, generates checksums and privacy-safe build
   metadata, and archives the result.
10. A human approves draft creation before the release credential is rebound
    for final publication; earlier release-state and previous-installer reads
    use separate bounded bindings.
11. Jenkins creates or updates only a draft prerelease and refuses published
    release replacement.
12. The maintainer performs the documented real-platform checks.
13. A protected promotion workflow verifies the reviewed evidence and
    publishes an immutable prerelease.

## Staged implementation

Each stage requires a fresh worktree, branch, open-pull-request, and task
inventory before edits begin. Each stage should remain independently
reviewable and revertible.

The Jenkins foundation implements exact-commit validation, credential
separation, complete routine checks, Windows/Linux snapshots, draft-only
publication, deterministic package counts, checksums, and build metadata.
Remaining stage text describes follow-up work and historical rationale; where
it conflicts with [Jenkins operations](jenkins.md), the implemented operations
document is authoritative.

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

**Status:** Superseded by publication-credential-free complete deterministic
Jenkins validation for every discovered pull request.

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

**Status:** Partially implemented. Boundary audits, Windows Clippy, locked Rust
resolution, package counts, and privacy-safe build metadata are active.
Append-only migration comparison remains proposed.

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
- use deliberately selected runner-image versions and record their identities;
  and
- add stronger Linux package-structure checks while retaining real
  operating-system startup checks as a manual prerelease boundary where
  automation is not trustworthy. A future macOS release requires a separate
  support decision.

### Stage 6 — optional Hoardmind conveniences

The detailed candidate architecture, safety boundaries, implementation stages,
and validation plan are recorded in the
[local review automation and bounded Hoardmind delegation plan](local-review-automation.md).
This section remains the CI/CD programme boundary for those proposed helpers.

ForgeAI is a separate implemented Jenkins exception, not a Hoardmind
convenience. It uses a controller-configured authenticated endpoint and a
bounded repository-generated packet with screened pipeline/dependency inputs,
runs only after deterministic work, and cannot gate or enter the protected
release job.

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
- any future macOS release, signing, or notarization;
- Tauri updater signing or automatic updates;
- stable releases;
- native Sentry PDB, dSYM, or ELF debug-information upload;
- public telemetry activation or embedded runtime DSNs;
- live OnAir behaviour or authenticated integration tests;
- live simulator certification;
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

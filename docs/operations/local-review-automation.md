# Local review automation and bounded Hoardmind delegation plan

**Status:** Stage 1 implemented; Stages 2–6 remain proposal only

**Reviewed baseline:** 19 July 2026

**Authority:** Stage 1 implements the deterministic evidence inventory described
below. This document does not authorize a later-stage implementation patch,
model invocation, hosted workflow, GitHub setting change, external request,
generated contribution, version change, tag, release, merge, signing operation,
or publication.

## Purpose

WyrmGrid already has strong deterministic gates and a bounded optional local-AI
runner. This plan joins those pieces into a repeatable local review process that
uses ordinary tooling for discovery and proof, Hoardmind for suitable bounded
drafting and triage, and ChatGPT/Codex semantic review only when its expected
benefit is high or a critical project boundary is involved.

The desired recurring path is:

1. inspect a selected working-tree, commit, or diff scope mechanically;
2. produce source-bound evidence and candidate findings without using AI;
3. let the maintainer select the relevant candidates and required gates;
4. prepare a small sanitized packet for one existing Hoardmind task when useful;
5. validate local-model output structurally and treat it as untrusted review
   evidence;
6. run deterministic checks against the actual repository state;
7. request ChatGPT/Codex semantic review only under the existing review budget;
   and
8. leave every compatibility, security, release, publication, and external
   mutation decision with an authorized person.

A routine non-critical change should be capable of completing this process with
zero ChatGPT/Codex tokens. The system must not relabel local-model tokens as
hosted tokens saved without separate hosted usage evidence.

This plan elaborates the local-helper proposal in the
[CI/CD hardening and enrichment plan](ci-cd-enhancement-plan.md#stage-6--optional-hoardmind-conveniences).
The implemented optional-AI boundaries remain defined by the
[optional local-AI development-task guide](../optional-ai/README.md).

## Design principles

- **Deterministic tools discover and prove.** A model does not decide which
  files changed, whether a command passed, whether a migration was preserved,
  or whether a fixture validates.
- **Hoardmind drafts from bounded evidence.** It receives only selected,
  sanitized source facts through one versioned task contract and one explicit
  invocation approval.
- **ChatGPT/Codex review is selective.** Valid low- or medium-benefit local-model
  output receives no duplicate frontier-model review. High-benefit and critical
  work retains the mandatory semantic-review path.
- **Humans retain authority.** Model output cannot approve, downgrade,
  implement, commit, push, merge, publish, release, change a version, or alter
  external settings.
- **Unknown means unavailable or escalated.** Missing tools, stale evidence,
  unclassified paths, ambiguous criticality, or failed sanitization never
  become an assumed pass.
- **WyrmGrid remains AI-independent.** Every deterministic command, build,
  contribution, and release path must remain usable without Hoardmind or any
  other AI.
- **Automation should reduce complexity.** Shared hashing, path handling,
  sanitization, evidence formatting, and result validation should have one
  tested implementation when their common contract is clear.

## Review routing

| Work class                                              | Deterministic checks                   | Hoardmind                     | ChatGPT/Codex                | Human decision                                   |
| ------------------------------------------------------- | -------------------------------------- | ----------------------------- | ---------------------------- | ------------------------------------------------ |
| Mechanical inventory or validation                      | Required                               | Not needed                    | Not needed                   | Reviews unexpected findings                      |
| Low- or medium-benefit drafting                         | Required                               | Optional bounded task         | No duplicate semantic review | Reviews and decides whether to use the draft     |
| High-benefit or broad semantic work                     | Required                               | Optional bounded first pass   | Required bounded review      | Required                                         |
| Critical boundary                                       | Required, with boundary-specific gates | Optional; never authoritative | Required                     | Required, including any separate approval policy |
| Release, tag, signing, publication, or external setting | Required complete evidence             | Curation only where permitted | Required                     | Explicit authorization required                  |

Critical boundaries include security, privacy, legal meaning, credentials,
authorization, cryptography, destructive or data-loss behaviour, database
migrations, protocol or schema compatibility, breaking-change and
semantic-version decisions, releases, tags, CI/CD, signing, installer identity,
live-provider support claims, and optional-AI or repository governance. A path
classifier can escalate work into this class but cannot prove that other work is
non-critical from filenames alone.

## Proposed local architecture

### One cohesive command surface

Stage 1 uses one dependency-light Node ESM entry point, following the
repository's existing tested script conventions:

- `scripts/local-review.mjs` — Git inventory, path and hash evidence, runtime
  evidence validation, local output, and human-readable summary;
- `scripts/local-review.test.mjs` — unit and integration-style tooling tests;
  and
- a shared safety helper only when extracting the already-duplicated hashing,
  secret-pattern, path-scope, or Markdown-heading rules produces a smaller and
  clearer authoritative implementation in a later stage.

Do not introduce a service, daemon, database, network listener, opaque agent,
or new application crate. The helper is local development tooling and is not
part of the WyrmGrid desktop application, plugin platform, CI requirement, or
release runtime.

The Stage 1 command is implemented:

```text
npm run review:inventory -- --base <git-ref>
```

`--base` is optional. `--output` may select a new directory only beneath
`.wyrmgrid-local/`; otherwise the command creates a timestamped review
directory. Exit status `0` means the requested inventory evidence was
available, `2` means an evidence bundle was written with one or more unavailable
Git facts, and `1` means the inventory itself failed. None of those statuses is
semantic approval.

The remaining candidate commands make the later-stage interface concrete and
do not exist yet:

```text
npm run review:validate -- --scope changed
npm run review:ready
npm run review:packet -- --task <task-id> --select <candidate-ids>
npm run review:verify-output -- --evidence <path> --draft <path>
```

The helper should use fixed argument arrays with Node's process APIs, not build
shell command strings from filenames, diffs, model text, logs, or environment
variables. Git path lists should use NUL-delimited output so spaces and Unicode
paths remain data rather than command syntax.

### Local output layout

All generated working evidence remains under the already ignored
`.wyrmgrid-local/` directory. Stage 1 writes:

```text
.wyrmgrid-local/review/<run-id>/
  evidence.json
  summary.md
```

Later stages may add:

```text
.wyrmgrid-local/review/<run-id>/
  receipts/
  packets/
  model-output/
```

The JSON evidence format is versioned by
[`local-review-evidence-v1.schema.json`](../../schemas/local-review-evidence-v1.schema.json)
and has a sanitized
[`version-1 fixture`](../../schemas/fixtures/local-review-evidence-v1.json).
The runtime validator rejects unknown fields, identities, classifications,
privacy claims, or inconsistent counts. The human-readable summary is derived
from the same validated structured evidence rather than maintained as a second
source of truth.

### Evidence identity

The Stage 1 evidence run records only privacy-reduced facts needed to identify
or invalidate the selected source snapshot:

- repository-relative selected paths;
- Git base and head identities when available;
- tracked, untracked, renamed, deleted, binary, and submodule state;
- SHA-256 hashes for selected source and configuration files;
- deterministic rule identifiers and candidate identifiers;
- critical-boundary flags and the rule or explicit maintainer classification
  that raised them; and
- explicit unavailable states when Git or selected file evidence cannot be
  established.

Later stages may add relevant lockfile, toolchain, formatter, audit-rule, and
task-contract hashes; fixed command receipts; platform facts; and packet or
local-model-output hashes. Those future fields require their own implemented
schema and compatibility decision.

Do not record environment dumps, credentials, raw provider payloads, databases,
personal data, unrelated source text, full failure logs, or personal absolute
paths. A receipt records that a check ran; it does not establish semantic
approval.

### Evidence version-1 compatibility decision

Version 1 is an internal maintainer-tooling snapshot, not an application,
plugin, Bridge, migration, or public provider contract. It is nevertheless
strictly versioned because later local helpers may consume it. Producers write
the exact version-1 kind and critical-rule-set identity; consumers must reject
unknown schema versions, kinds, rule sets, fields, or changed meanings.

Adding, removing, renaming, or redefining evidence fields, enum values,
classification semantics, privacy claims, or the critical rule set requires a
new evidence version and an explicit read, reject, or migration decision.
Existing ignored evidence is never silently upgraded and never becomes release
or compatibility authority. It may be deleted and regenerated from current
source. Existing optional-AI task, profile, metrics, protocol, application, and
database versions are unchanged by Stage 1.

### Classification behaviour

The Stage 1 classifier combines conservative reviewed path rules with evidence
availability. Later stages may add dependency facts and explicit diff signals
only with equivalent tests. It returns one of:

- `routine-candidate` — no deterministic critical trigger was found, but a
  human still reviews the scope;
- `critical-candidate` — at least one mandatory review trigger was found; or
- `classification-required` — evidence is insufficient or the path/rule is
  unknown.

Untrusted branch names, commit text, pull-request text, source comments, model
output, and contributor-supplied labels must not reduce the classification. A
maintainer may escalate any scope. Resolving `classification-required` or any
proposed downgrade of a deterministic critical trigger must be explicit,
reasoned, and governed by the future approved policy rather than silently
accepted by the tool.

## Deterministic automation catalogue

The common evidence layer can support the following audits without asking a
model to inspect the whole repository.

| Area                           | Mechanical discovery or proof                                                                                                                                                                 | Result type                                                       |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Code comments                  | Public contracts without documentation, unsafe blocks without nearby safety rationale, unit conversions, provider translation points, deliberate fallback handling, and complexity thresholds | Advisory candidate inventory                                      |
| Change-aware validation        | Map selected paths and dependency facts to the smallest useful fast checks, then retain a separate complete readiness gate                                                                    | Command plan and receipts                                         |
| Tests                          | Identify changed production areas, nearby dedicated tests, changed decision coverage, and absent success, boundary, failure, unavailable-data, or regression categories                       | Advisory test-gap inventory                                       |
| Fixtures                       | Match versioned schemas and protocols with validators, canonical fixtures, and required valid and invalid cases                                                                               | Critical candidate inventory and validator receipts               |
| Documentation                  | Check repository-relative links, command names, version markers, declared availability, and selected source-of-truth relationships                                                            | Drift candidates; never silent rewriting                          |
| Failures                       | Remove personal paths and disallowed values, bound output, preserve the first useful error and exit status, and deduplicate repeated lines                                                    | Sanitized failure evidence                                        |
| Migrations                     | Compare released migrations with the approved release identity and require only new contiguous append-only files                                                                              | Critical pass, fail, or unavailable result                        |
| OnAir boundary                 | Detect write-oriented request methods, generic unbounded request paths, and raw OnAir type use outside the adapter boundary                                                                   | Critical candidate or hard boundary failure once rules are proven |
| Interface boundaries           | Extend localization and desktop-command identifier checks; inventory likely business orchestration in Svelte or Tauri without claiming semantic proof                                         | Hard identifier checks plus advisory architecture candidates      |
| Plugins                        | Validate known capabilities, deny-by-default declarations, bounded origins, manifest/schema/fixture parity, and protocol-version markers                                                      | Critical pass, fail, or compatibility candidate                   |
| Privacy                        | Detect high-confidence credential patterns, personal absolute paths, prohibited raw-payload logging, and unsafe fixture fields                                                                | Fail-closed finding requiring human review                        |
| Complexity                     | Track file, component, and function growth against a reviewed baseline                                                                                                                        | Trend report, not an automatic refactor order                     |
| Localization and accessibility | Inventory direct user-facing strings, missing typed catalogue mappings, keyboard-state cases, reduced-motion cases, and localized expansion coverage                                          | Cohesive surface candidates                                       |
| Release readiness              | Assemble exact source identity, complete gate receipts, migration state, expected packages, documentation state, and reviewed changelog evidence                                              | Critical evidence bundle; never release authority                 |

Only mature, low-noise rules should become blocking gates. Subjective rules
start as reports. A comment or complexity finding should ask whether clearer
naming, extraction, or separation would remove the need for commentary before
it proposes more text.

## Bounded Hoardmind use

### Existing task mapping

No new task contract is required for the first implementation. The shared
helper can prepare reviewed packets for existing version-1 tasks:

| Deterministic evidence                                      | Existing Hoardmind task   | Permitted use                                                                             |
| ----------------------------------------------------------- | ------------------------- | ----------------------------------------------------------------------------------------- |
| Selected diff and affected-component facts                  | `change-impact-v1`        | Challenge the component, test, documentation, compatibility-flag, and changelog inventory |
| Approved behaviour and test-gap candidates                  | `test-matrix-v1`          | Draft success, boundary, failure, unavailable-data, and regression cases                  |
| Approved schema plus sanitized synthetic fixture            | `fixture-variants-v1`     | Draft valid and invalid synthetic variants                                                |
| Confirmed change facts plus selected documentation excerpts | `docs-sync-v1`            | Draft narrow documentation synchronization candidates                                     |
| Sanitized bounded command failure                           | `failure-triage-v1`       | Cluster symptoms and propose non-destructive local checks                                 |
| One approved objective and selected source/test evidence    | `implementation-patch-v1` | Draft one narrow review-only textual patch                                                |
| Reviewed release facts and changelog scope                  | `release-curation-v1`     | Draft the required changelog categories without deciding release readiness                |

Comment work initially uses deterministic candidate discovery followed by a
small `implementation-patch-v1` packet for one approved file or cohesive area.
A dedicated comment task should be considered only after actual use shows that
the existing contract cannot express the required boundary. Creating or
changing a task contract is optional-AI governance and requires critical
review.

### Packet preparation

Packet generation should:

1. require explicit candidate identifiers and exact allowed paths;
2. re-read and hash the current selected source immediately before creating the
   packet;
3. include only the minimum source excerpts, tests, contracts, and
   documentation needed for the selected task;
4. convert personal absolute paths to repository-relative labels;
5. reject credentials, raw provider payloads, personal data, databases, crash
   dumps, unrelated source, oversized evidence, binary content, path traversal,
   symlink escape, and unsupported encodings;
6. state confirmed facts, uncertainty, exclusions, required headings, and
   required deterministic validation explicitly;
7. display a human-readable packet preview and source hashes; and
8. stop without invoking a model.

The maintainer then runs the existing task command with `--approve-once`. The
helper must never automatically feed a Hoardmind result into another task. A
later packet is rebuilt from confirmed repository source and deterministic
evidence, not copied from prior model prose.

### Output handling

The existing runner remains responsible for task/profile validation, output
heading validation, exact local token metrics, and adapter boundaries. The
review helper may additionally bind the draft to the evidence and packet hashes
and report:

- structurally valid or invalid;
- current or stale against selected source hashes;
- low, medium, high, or critical expected review benefit after human
  classification; and
- applicable deterministic commands that still need to run.

It must not say that model output is correct, that tests passed, or that a
change is safe. Patch application remains manual unless the existing
hash-bound generated-contribution workflow is explicitly selected. A wholly
assistant-generated published patch retains all generated-contribution and
human landing requirements.

## Incremental cache

Caching should avoid repeated local work without creating false evidence.

A deterministic result is reusable only when its cache key covers every
material input, including:

- selected source hashes and Git identities;
- audit-rule and script hashes;
- configuration, schema, fixture, manifest, and lockfile hashes;
- relevant compiler, runtime, formatter, linter, validator, and target versions;
- platform and architecture where behaviour differs; and
- an expiry policy for advisory, dependency, or other time-sensitive evidence.

Source edits during a run make the affected result stale. Interrupted or
partially written results are unavailable, not failed or passed. Writes should
be atomic. The summary must distinguish `ran`, `cached`, `failed`, `stale`,
`skipped by policy`, and `unavailable`.

Cached fast checks cannot satisfy a complete release or readiness gate unless
the approved policy explicitly permits that exact evidence class. Live
provider certification, external settings, platform signing, installer
behaviour, and other environment-dependent checks remain outside this cache.

## Change-aware local validation

Two validation modes should remain distinct:

- **Changed-scope validation** gives fast feedback by selecting checks relevant
  to the current bounded diff. It is an optimization, not a declaration that
  the repository is ready.
- **Complete readiness validation** runs every locally required gate from the
  testing and release policies for the intended handoff. It records failures
  and unavailable platform checks honestly.

The command registry should contain fixed reviewed commands rather than accept
arbitrary command text from packets or model output. Initial entries can wrap
existing formatting, Rust, frontend, tooling, boundary, dependency, desktop,
and provider checks. Command selection rules require tests, and an unknown
changed path should broaden or block the plan rather than quietly reduce it.

## Safety and threat-model requirements

Implementation changes the optional-AI development boundary and therefore
requires a corresponding threat-model update before the helper is considered
complete. The review must cover at least:

- prompt injection embedded in source, docs, fixtures, logs, filenames, commit
  messages, and model output;
- command injection and unsafe quoting on Windows;
- path traversal, symlink or junction escape, alternate data streams, and files
  outside the approved repository root;
- time-of-check/time-of-use changes between evidence, packet creation,
  validation, and patch application;
- credential, personal-data, raw-provider-payload, database, crash-dump, and
  personal-path leakage;
- maliciously large, binary, malformed, or adversarially encoded inputs;
- false cache hits, partial writes, reused failures, and stale security data;
- classification downgrades through untrusted metadata;
- model claims being mistaken for executed checks or compatibility evidence;
- automatic chaining or durable model memory; and
- accidental expansion of the GitHub App, CI, release, merge, or publication
  authority.

The implementation must not expose a general command runner, arbitrary file
reader, repository-writing agent, network proxy, or GitHub credential to
Hoardmind. Failure sanitization should happen before packet creation, and the
unsanitized input should not be copied into local metrics or reports.

## Implementation stages

Each stage should be a separate reviewed change with dedicated tooling tests,
documentation synchronization, and an `[Unreleased]` changelog decision.

### Stage 1 — evidence inventory

**Implemented 19 July 2026.**

- Define the reviewed scope, critical-path rules, evidence fields, and local
  directory layout.
- Implement NUL-safe Git inventory, repository-root enforcement, content
  hashing, candidate identifiers, and human-readable summaries.
- Record clean, dirty, untracked, renamed, deleted, binary, and unavailable
  states without invoking Hoardmind.
- Add an optional-AI threat-model update and explicit internal compatibility
  decision for the evidence format.

### Stage 2 — validation registry and receipts

- Add a fixed allowlisted registry for existing local commands.
- Implement changed-scope selection, complete-readiness selection, exact
  receipts, atomic writes, and cache invalidation.
- Begin with existing format, tooling, boundary, Rust, frontend, dependency,
  Tauri, and simulator-provider gates.
- Do not enable cached results for release authority.

### Stage 3 — deterministic boundary audits

- Implement append-only released-migration verification first because the gap
  is already confirmed.
- Add schema/fixture/validator parity and conservative OnAir read-only/raw-type
  containment checks.
- Extend identifier and manifest audits where rules can be exact.
- Keep Tauri thinness, Svelte responsibility, comment quality, and complexity
  as advisory findings until a reliable mechanical rule is proven.

### Stage 4 — bounded packet preparation

- Prepare task-specific packets only for the existing version-1 contracts.
- Reuse one authoritative implementation of hashing, path scoping,
  sanitization, size limits, and secret-like rejection where practical.
- Require packet preview and one-invocation approval.
- Bind packets and outputs to evidence hashes without applying output.

### Stage 5 — comments, tests, fixtures, docs, and failure workflows

- Add comment and complexity candidate inventories.
- Add test-category and fixture-coverage candidate reports.
- Add documentation-link, command-name, and status-claim drift checks.
- Add bounded failure sanitization and deduplication.
- Pilot each workflow on one cohesive non-critical area before broadening it.

### Stage 6 — readiness evidence

- Assemble exact local readiness evidence from confirmed receipts.
- Integrate the deterministic evidence with the existing release-curation
  packet preparation without granting release authority.
- Coordinate with the separately governed CI/CD stages for migration policy,
  package manifests, exact release source identity, supply-chain evidence, and
  hosted release checks.

## Validation plan

Tooling tests should be physically separate from production scripts and cover:

- clean, dirty, staged, unstaged, untracked, renamed, deleted, binary, Unicode,
  and space-containing paths;
- an absent Git reference, unavailable tool, interrupted command, non-zero exit,
  truncated output, and source changing during execution;
- repository-root, traversal, symlink/junction, and unsupported-file rejection;
- stable candidate identifiers across line movement and invalidation after
  relevant source changes;
- critical-path escalation, unknown-path escalation, and resistance to
  downgrade text in commits, source, logs, or model output;
- cache hits with identical inputs and misses after every material source,
  configuration, lockfile, toolchain, platform, or expiry change;
- atomic receipt creation and recovery from incomplete local files;
- secret-like, personal-path, raw-payload, oversized, binary, and malformed
  packet rejection;
- exact task headings, allowed task identifiers, packet hashes, output hashes,
  and stale-output detection;
- no automatic model invocation during inventory or packet preparation;
- no automatic task chaining, patch application, commit, push, PR, release, or
  external mutation;
- a complete deterministic path when Hoardmind is absent; and
- unchanged behaviour of the existing optional-AI runner, contribution broker,
  landing guard, localization audit, and desktop-command audit.

Applicable stages must also run repository formatting, tooling tests, Rust and
frontend gates, dependency audits, desktop/provider checks, and threat-model
review according to the paths they change. A generated test matrix or failure
diagnosis is never a substitute for these checks.

## Efficiency evidence

The helper may report privacy-reduced efficiency facts such as:

- deterministic candidates found, accepted, rejected, and deferred;
- commands run, reused, failed, stale, or unavailable;
- packet byte size and selected source count;
- exact local tokens and timing reported by the existing optional-AI runner;
- how many low/medium tasks completed without frontier semantic review; and
- how many high/critical tasks were escalated.

Do not store prompts, source excerpts, model responses, credentials, personal
paths, or raw failures in aggregate metrics. Do not infer hosted token, time,
energy, or monetary savings without separately measured hosted evidence.
Optimize only after several real runs reveal packet-preparation cost, false
positive rate, cache effectiveness, and review usefulness.

## Non-goals

This plan does not propose:

- an autonomous repository-scanning or repository-writing model;
- automatic model invocation or model-to-model chaining;
- AI in WyrmGrid runtime, builds, tests, CI, contribution requirements, or
  release publication;
- an authenticated, LAN, hosted, or tool-enabled Hoardmind adapter;
- a model deciding business rules, security, privacy, legal meaning,
  compatibility, migrations, versions, live-provider behaviour, or releases;
- automatic suppression, autofix, patch application, commit, push, PR, review,
  merge, tag, signing, publication, or GitHub-setting changes;
- replacement of complete local gates with diff-only checks;
- coverage quotas, comment quotas, or complexity scores that reward superficial
  changes; or
- broad context collection merely because it is available.

## Completion conditions

The programme is complete only when:

- each implemented command is documented, tested, deterministic, and usable
  without Hoardmind;
- every durable evidence or receipt format has an explicit compatibility
  decision;
- critical classification, sanitization, path, cache, and command boundaries
  fail closed;
- packet creation never invokes a model and every model run still requires one
  explicit approval;
- low/medium Hoardmind results can avoid redundant ChatGPT/Codex review without
  bypassing human review or deterministic gates;
- high-benefit and critical work reliably reaches the required semantic and
  human review path;
- no model output is treated as test, compatibility, release, or approval
  evidence;
- the threat model, testing strategy, optional-AI guide, development guidance,
  and changelog match implemented behaviour; and
- the maintainer explicitly authorizes each implementation stage separately.

# Hoardmind hosted-platform prototype work orders

## Status

Prepared for maintainer activation on 2026-07-20, with three bounded smoke
activations recorded below. These remain prototype propositions, not authority
to publish, merge, deploy, create accounts, accept uploads, change a protocol or
schema, or handle real user data.

Hoardmind can now create projects and edit files directly. That capability does
not change WyrmGrid's review or publication authority. The existing optional-AI
runner remains review-only, and its base prompt still prohibits tool and file
access. Direct project work therefore uses isolated, gitignored prototype roots
and returns evidence for a later human decision. It does not silently extend the
runner's task contracts.

The three initial propositions are:

| ID            | Prototype                    | Risk                     | Primary model     |
| ------------- | ---------------------------- | ------------------------ | ----------------- |
| `HP-PROT-001` | Static website               | Medium                   | `qwen3.6:35b`     |
| `HP-PROT-002` | Catalogue presentation       | Medium                   | `qwen3.6:35b`     |
| `HP-PROT-003` | Inert package fixture corpus | High, security-sensitive | `qwen3-coder:30b` |

Their local project roots are under
`.wyrmgrid-local/hoardmind-projects/hosted-platform/`. Nothing under that root
is a WyrmGrid source, test, release, or deployment artifact until a person
reviews it and explicitly approves a separate integration change.

## Common execution contract

### Authority

For an individually activated work order, Hoardmind may:

- read the exact repository evidence listed by that work order;
- create and edit files only within its assigned local project root;
- install dependencies only inside that local project when the work order
  permits them;
- run non-destructive local formatting, checking, test, build, and inspection
  commands inside that project; and
- return a final handoff with the model identity, exact project root, file list,
  commands, observed results, unresolved questions, and recommended next step.

Hoardmind may not:

- edit the WyrmGrid repository, another worktree, another prototype, user
  directories, or external systems;
- read or receive credentials, API keys, tokens, databases, portable backups,
  raw provider payloads, personal data, signing material, production logs, or
  unrelated source context;
- create commits, branches, pull requests, tags, releases, workflow runs,
  deployments, DNS records, accounts, mail, storage buckets, or firewall rules;
- select dependencies for production, decide compatibility, approve legal or
  security meaning, alter WyrmGrid architecture, or claim a service is live;
- fetch or render remote fonts, images, scripts, styles, analytics, trackers,
  advertisements, map tiles, or provider data in the prototype; or
- use one model's response as another model's packet. Every comparison starts
  from the confirmed work order and deterministic project evidence.

### Isolation and scheduling

- Give each work order its own local project directory and process set.
- Do not run two work orders that install packages, start servers, or write the
  same cache concurrently.
- The main WyrmGrid worktree is read-only evidence and may remain dirty. A
  prototype must not interpret an unrelated working-tree change as authority.
- Pin local dependencies in the prototype, but do not copy its lockfile into the
  WyrmGrid repository. Production dependency adoption remains a separate
  reviewed decision.
- Use synthetic names and data visibly marked `EXAMPLE`, `TEST`, or `DEMO`.
- Stop rather than broadening the allowed root or inventing a missing product,
  schema, brand, legal, security, or compatibility decision.

### Required final handoff

Every result ends with:

1. objective completed or blocked;
2. exact model and runtime configuration;
3. created and modified files;
4. dependency inventory and licences reported by the selected package manager;
5. commands run and their actual exit status;
6. successful, boundary, failure, and unavailable-state evidence;
7. accessibility or sanitization evidence applicable to the slice;
8. unresolved decisions and assumptions;
9. confirmation that no forbidden path, credential, network service, commit, or
   deployment was touched; and
10. a recommendation to discard, revise, or propose a separately reviewed
    integration packet.

Claims without deterministic evidence remain unverified.

## Model research and routing

### Observed local capacity

The development host observation on 2026-07-20 was:

- NVIDIA GeForce RTX 4070 Ti with 12,282 MiB VRAM;
- 63.1 GiB system RAM;
- Ollama 0.32.0;
- installed `qwen3.6:35b` at approximately 23 GB,
  `qwen3-coder:30b` at approximately 18 GB, and `gpt-oss:20b` at approximately
  13 GB; and
- a previously successful Hoardmind profile using `qwen3-coder:30b` with an
  8,192-token context and partial GPU offload.

This is the maintainer's development machine, not the dedicated-server
specification. A server model allocation must be measured separately.

### Hugging Face evidence

The model recommendations use official Hugging Face model cards queried with
`hf` CLI 1.24.0. Popularity and vendor-reported benchmarks are discovery
evidence, not proof of performance on WyrmGrid.

| Model                                                                                       | Card evidence                                                                                                                                                                                                                                                                                    | Local conclusion                                                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Qwen3.6-35B-A3B](https://huggingface.co/Qwen/Qwen3.6-35B-A3B)                              | Apache-2.0; about 35.95B total and 3B activated parameters; native 262,144-token context; the card specifically describes frontend workflows, repository-level reasoning, agentic coding, tool use, and a frontend benchmark. Hub revision observed: `995ad96eacd98c81ed38be0c5b274b04031597b0`. | Best first candidate for the two presentation prototypes because it is already installed and its stated specialization matches frontend and repository work. Use a bounded context rather than the card maximum on current hardware. |
| [Qwen3-Coder-30B-A3B-Instruct](https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct)    | Apache-2.0; 30.5B total and 3.3B activated parameters; native 262,144-token context; the card targets agentic coding, browser use, tool calls, and repository-scale understanding. Hub revision observed: `b2cff646eb4bb1d68355c01b18ae02e7cf42d120`.                                            | Proven current Hoardmind default and best first candidate for deterministic generators, tests, manifests, and cross-file fixture bookkeeping.                                                                                        |
| [GPT-OSS-20B](https://huggingface.co/openai/gpt-oss-20b)                                    | Apache-2.0; approximately 21B total and 3.6B activated parameters; configurable reasoning effort, tool use and structured outputs; the card states that its released quantization runs within 16 GB memory. Hub revision observed: `6cee5e81ee83917806bbde320786a8fb61efebee`.                   | Useful installed challenger for an independently prepared adversarial test-matrix pass. It does not replace the required human and Codex review of security-sensitive fixtures.                                                      |
| [Devstral Small 2 24B](https://huggingface.co/mistralai/Devstral-Small-2-24B-Instruct-2512) | Apache-2.0; 24B parameters; 256K context; its card targets tool-driven codebase exploration and multi-file software-engineering edits.                                                                                                                                                           | Sensible future bake-off candidate, but it is not currently installed. Do not spend download time or storage until the installed candidates fail the slice gate or a separate comparison is approved.                                |

Qwen3-Coder-Next was not selected for this machine. The Hub reports roughly
79.7B parameters, making a useful quantization and context allocation too close
to the 63.1 GiB system-memory ceiling for a first prototype. A larger model that
swaps heavily is not automatically a better agent.

### Recommended runtime profiles

These are starting points for a direct Hoardmind project, not changes to the
versioned optional-AI profile schema:

| Use                                   | Model             | Context | Reasoning                                     | Sampling                                                                                                 |
| ------------------------------------- | ----------------- | ------: | --------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Static website                        | `qwen3.6:35b`     |  32,768 | Thinking enabled for implementation decisions | Begin near the model-card coding guidance, then lower randomness if the project shows unstable diffs     |
| Catalogue presentation                | `qwen3.6:35b`     |  32,768 | Thinking enabled                              | Same starting point; require deterministic fixtures and checks rather than judging appearance from prose |
| Package fixture implementation        | `qwen3-coder:30b` |  16,384 | Keep the currently proven mode first          | Temperature `0.1`, seed `42`; prioritize reproducible generators and tests                               |
| Independent fixture-matrix challenger | `gpt-oss:20b`     |  16,384 | High reasoning                                | Request structured cases only; do not grant write access during this comparison                          |

The production choice is made by a small slice-specific bake-off, not solely by
the table. Give each candidate the same confirmed micro-task in a disposable
copy and score:

- deterministic checks and acceptance cases: 50%;
- scope discipline and absence of invented decisions: 20%;
- maintainability and separation of concerns: 15%;
- elapsed time, exact tokens and resource observations: 10%; and
- dependency restraint and licence clarity: 5%.

Discard any candidate that touches a forbidden path, fabricates a test result,
introduces an unapproved network request, or cannot explain its dependency
changes, regardless of score.

## HP-PROT-001: Static website prototype

### Objective

Create a non-public, mostly static SvelteKit prototype that demonstrates a
credible WyrmGrid public presence without accounts, analytics, a database, an
API, remote assets, or deployment configuration.

### Project root

`.wyrmgrid-local/hoardmind-projects/hosted-platform/static-website/`

Hoardmind may write only inside that directory. The WyrmGrid repository is
read-only evidence.

### Selected evidence

- [Project brief](../project-brief.md)
- [Architecture overview](../architecture/overview.md)
- [ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
- [Hosted-platform implementation plan](../operations/hosted-platform.md)
- [Documentation index](../README.md)
- existing desktop semantic colours, spacing and typography only as visual
  reference; do not copy application business logic or private data

### Required prototype

- Use a standalone SvelteKit project configured for static output.
- Provide Home, Features, Documentation, Downloads, Security, Legal and Aerie
  preview routes. The Downloads route uses unmistakably non-live placeholders
  and links to no fabricated release.
- Create a reusable presentational shell, navigation, footer, content sections,
  cards, notices, status labels and call-to-action components rather than
  duplicating page markup.
- Present local-first operation, provenance, offline behaviour,
  out-of-process plugins and optional hosted services accurately.
- Make Aerie visibly a future proposal. Do not present accounts, uploads,
  signing, backups or synchronization as available.
- Use local placeholder art or CSS decoration only. No generated project logo
  replaces the existing identity, and no third-party brand suggests approval.
- Support keyboard navigation, visible focus, semantic landmarks, skip link,
  correct heading order, reduced motion, high zoom, narrow screens and
  high-contrast preferences.
- Render a useful not-found page and build-time failure messages that do not
  expose local paths or pretend to be production diagnostics.
- Include a concise README naming commands, structure, dependencies, licences,
  limitations and the path to discard the prototype.

### Non-goals

- No authentication, forms that submit, cookies, telemetry, analytics, CMS,
  comments, search service, mail, payment, CDN, DNS, TLS or deployment work.
- No catalogue API, package install, upload, vault, synchronization, OnAir call,
  simulator integration or provider data.
- No edit to WyrmGrid packages, workspaces, lockfiles, source, legal text,
  assets, workflows or documentation.

### Deterministic gates

- clean dependency installation from the local lockfile;
- Svelte type checking, formatting and production static build;
- no remote URL in generated HTML, CSS, JavaScript or assets except plain,
  deliberately reviewed outbound documentation links;
- no cookie, local-storage, service-worker, analytics or form-submission code;
- an automated route and link check;
- automated accessibility checks where the selected local dependency permits,
  plus recorded manual keyboard and 200% zoom observations; and
- useful rendering with JavaScript disabled for the informational content.

### Model routing

Use `qwen3.6:35b` first. If its micro-task fails scope discipline or deterministic
checks, compare `qwen3-coder:30b` from the same confirmed work order. Do not
install another model for this slice without recording why both installed
candidates failed.

### Exit

The result is a local visual and structural proposal. Integration requires a
separate human-approved decision covering the real repository location,
dependency changes, brand assets, reviewed copy, licence notices, test plan and
deployment boundary.

## HP-PROT-002: Catalogue presentation prototype

### Objective

Create a standalone SvelteKit presentation prototype for anonymous, read-only
Aerie discovery using only synthetic fixtures. Demonstrate how trust,
compatibility, permissions, licensing, publisher identity, yanking, revocation,
unavailable data and empty results are communicated without inventing an API or
package contract.

### Project root

`.wyrmgrid-local/hoardmind-projects/hosted-platform/catalogue-presentation/`

Hoardmind may write only inside that directory. It must not import the static
website prototype or depend on another model's output. A later human review may
decide whether useful presentation work is combined.

### Selected evidence

- [ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
- [Hosted-platform implementation plan](../operations/hosted-platform.md)
- [Hosted-platform licensing register](../legal/hosted-platform-licensing.md)
- [Plugin platform overview](../plugins/overview.md)
- [Plugin protocol version 1](../plugins/protocol-v1.md)
- [Threat model](../security/threat-model.md), limited to the proposed hosted
  controls and residual hosted/plugin risks

### Required prototype

- Use a standalone SvelteKit static project with a fixture-backed repository
  adapter. Pages and components never import fixture JSON directly; they consume
  a small presentation interface so future transport remains replaceable.
- Provide package browse and detail views with synthetic search, package-kind,
  compatibility, licence and status filters; deterministic sort; result count;
  and clear reset.
- Represent data-only theme, language pack, ordinary plugin and native-provider
  examples as different trust classes. Do not imply that one approval covers
  another.
- Present stable synthetic publisher identity separately from display name and
  signing-key facts.
- Distinguish publisher signature, WyrmGrid repository approval, scanner result,
  moderation state, compatibility and user permissions. None is labelled
  “safe,” “certified” or guaranteed.
- Show requested permissions and network origins before any mock install
  affordance. The affordance is disabled and labelled presentation-only.
- Include published, awaiting-review, yanked, revoked, incompatible, permission-
  changed, unknown-licence, missing-data and stale-metadata examples.
- Include loading, empty, filtered-empty, malformed-fixture, unavailable and
  retry-without-network states without manufacturing fallback facts.
- Keep licence expression, notices availability, source location and package
  rights warnings visible on detail views.
- Make lists, filters, status and details usable by keyboard, screen reader,
  reduced motion, high zoom and narrow screens.
- Include a fixture dictionary explaining that all identities, versions,
  digests, dates, licences and URLs are synthetic and non-authoritative.

### Non-goals

- No live API, OpenAPI file, package schema, signing implementation, account,
  upload, moderation action, review queue, install, update, payment, download,
  analytics, storage, database or deployment.
- No claim that the current plugin protocol is production-hardened or that
  Aerie exists.
- No edit to official schemas, protocol files, legal documents, threat model,
  WyrmGrid source, dependencies or the static website prototype.

### Deterministic gates

- clean dependency installation, Svelte checking, formatting and static build;
- fixture parsing through the prototype adapter with stable invalid-fixture
  outcomes;
- component tests for filters, sorting, reset, status labels and missing data;
- dedicated tests proving yanked, revoked, incompatible and permission-changing
  packages are never presented as ordinary installable results;
- no remote request, form submission, cookie, persistent browser storage,
  service worker or enabled install/download action;
- route/link and accessibility checks plus recorded keyboard and 200% zoom
  observations; and
- a text inventory check rejecting “safe,” “certified,” “guaranteed,” “official
  publisher” and other unapproved trust claims.

### Model routing

Use `qwen3.6:35b` first because the official card specifically identifies
frontend workflows and repository reasoning. Use `qwen3-coder:30b` as the
fallback comparison if the first candidate over-designs the interface or fails
fixture and state tests.

### Exit

The result is presentation research only. A person extracts confirmed UX
requirements independently; the prototype does not become the catalogue API,
package schema, legal promise or production site by adoption.

## HP-PROT-003: Inert package fixture corpus

### Objective

Create a bounded, inert corpus prototype and generator that exercises the
archive and metadata hazards already named in ADR-0019 without defining the
future package schema or producing executable, weaponized or resource-exhausting
payloads.

### Project root

`.wyrmgrid-local/hoardmind-projects/hosted-platform/package-fixture-corpus/`

Hoardmind may write only inside that directory. This work is security-sensitive
and requires independent Codex and human review before any artifact or lesson
is integrated.

### Selected evidence

- [ADR-0019 package publication boundary](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
- [Hosted-platform archive and manifest validation plan](../operations/hosted-platform.md)
- [Hosted-platform licensing register](../legal/hosted-platform-licensing.md)
- [Threat model planned hosted controls](../security/threat-model.md)
- existing WyrmGrid fixture naming and test-location conventions as structural
  examples only; existing schemas do not define the Aerie package format

### Required prototype

- Produce a corpus catalogue with a unique case ID, category, intended invariant,
  construction method, expected prototype outcome, bounded size, digest and
  sanitization note for every generated artifact.
- Use a small standard-library generator and dedicated tests. Every binary
  artifact must be reproducible from reviewed text source; do not hand-edit
  opaque archives.
- Use inert UTF-8 text payloads only. Do not include executable formats, scripts
  intended to run, macros, bytecode, native code, shell commands, malware,
  antivirus test signatures, credentials, personal data or copied third-party
  packages.
- Cover harmless valid controls and proposed rejections for parent traversal,
  absolute paths, mixed separators, symbolic and hard links, device names,
  alternate data streams, case-folding collisions, duplicate entries,
  dangerous bidirectional controls, excessive path depth, excessive file count,
  compressed expansion, corrupt/truncated archives and digest mismatch.
- Represent large-size, expansion and file-count limits with small test ceilings
  or manifest metadata. Do not create a genuine disk-filling archive, sparse
  file, fork bomb, high-CPU payload or uncontrolled decompression bomb.
- Cover metadata states for missing manifest, multiple candidate manifests,
  unknown package kind, absent or malformed licence expression, missing notices,
  incompatible runtime, undeclared dependency, undeclared network origin,
  permission change, publisher-key change, awaiting review, yanked and revoked.
- Mark every metadata shape `PROTOTYPE-NONAUTHORITATIVE`. Fields and expected
  results are hypotheses derived from named risks, not schema decisions.
- Produce a decision report separating cases independent of package format from
  cases blocked on archive, canonical path, manifest, signing, compatibility or
  permission-contract approval.
- Record exact generator dependencies and licences. Prefer the language standard
  library and avoid adding an archive dependency unless a case cannot otherwise
  be represented.

### Non-goals

- No official package schema, protocol version, stable error code, cryptographic
  algorithm, signing metadata, validator implementation or compatibility
  decision.
- No fuzzing of production code, scanner execution, external upload, network
  request, server process, container escape test or operating-system exploit.
- No real package, publisher, licence dispute, key, signature, OnAir data,
  provider data, plugin code or vulnerability proof of concept.
- No edit to `schemas/`, production tests, protocol crates, legal/security docs,
  dependencies, migrations or workflows.

### Deterministic gates

- rebuilding the corpus twice produces byte-identical artifacts and catalogue
  digests;
- deleting generated artifacts and regenerating from source restores the same
  inventory;
- all artifacts stay within the work-order byte, file-count and execution-time
  ceilings recorded before generation;
- a content scan finds no executable headers, secrets, personal identifiers,
  network origins other than reserved synthetic examples, or antivirus test
  signatures;
- tests verify the generator cannot write outside its output root, even for
  traversal-labelled archive entries;
- every case has exactly one catalogue entry and every generated artifact is
  catalogued;
- no generated artifact is opened, imported, executed or extracted by the test
  host outside a disposable bounded directory; and
- the final report clearly labels expected outcomes as proposed until an
  approved package contract and real validator exist.

### Model routing

Use `qwen3-coder:30b` for the direct project because it is already proven in
Hoardmind and is specialized for cross-file coding work. Independently give
`gpt-oss:20b` a read-only, structured matrix prompt built from this work order;
compare category coverage manually without copying its response into the
generator packet. Any security-relevant addition must be traced back to the
confirmed threat model or explicitly approved as a new decision.

### Exit

The corpus remains inert prototype evidence. Integration waits for an approved
archive and package contract, stable validator categories, selected test
location, bounded production limits, licensing review and required security
review. Prototype fixtures that encode a rejected schema choice are discarded,
not grandfathered into compatibility.

## Live smoke activation record — 2026-07-20

The maintainer confirmed that BitLocker protection for `K:` was enabled and
that its recovery key was stored safely. They also approved the currently
commissioned Hoardmind Scribe base model, `qwen3.5:35b`, for the initial smoke
attempt. This was a one-run operational allowance, not a change to the model
recommendations above.

Offline Gate action and Scribe profile validators, followed by the focused live
Scribe-boundary validator, passed. One inert existing `index.html` seed was
placed in each protected presentation-project subdirectory. No package or model
download was started.

The Hearth Scribe chat did not invoke its attached workspace tools. Its first
response invented a placeholder file hash and simulated proposal identifiers;
after correction it reported `BLOCKED: TOOL UNAVAILABLE`. A candidate-only
retry also invented unsupported current-product claims about signing,
telemetry, plugin enforcement and synchronization. Those responses were
discarded and never submitted to Action Center.

The already-installed `qwen3-coder:30b` fallback then produced candidate
single-file smoke layouts from the unchanged work-order facts. Because the copy
touches product, security and legal meaning, Codex performed the required
critical-boundary review before proposal creation. Deterministic local checks
and loopback rendering were used as evidence; neither model was credited with
tests it did not run.

The commissioned proposal service created two exact, independently reviewable
and expiring Action Center records:

| Slice                        | Target                                                       | Candidate SHA-256                                                  | Proposal ID                                 | Proposal hash                                                      |
| ---------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------ | ------------------------------------------- | ------------------------------------------------------------------ |
| Static website smoke         | `wyrmgrid-hosted-platform/static-website/index.html`         | `2fe671148f0cdad31fbbe138d2869e73d16d5ae6ce03d32d21a70610702693b1` | `proposal-64fb1c7ef6effb8bc2654c39ef5b9fbd` | `d352c58afe92be99766e7dc3091b37b9a163d4108d8813cb82a487396ff8c1e4` |
| Catalogue presentation smoke | `wyrmgrid-hosted-platform/catalogue-presentation/index.html` | `f39296b6d772db8ba88c8f4f51598734bfbaa4765d385048cf2323c7516aae6d` | `proposal-9fcc140367c0d41281cec96e37176901` | `c76532405939f0ffc6ce96c696c441cca29fa50d4ed8eb25ff7650a2a55b0fee` |

The owner subsequently unlocked Action Center, reviewed and separately applied
both exact proposals. Each proposal reached `succeeded`. Commissioned
read-back and independent file hashing matched the candidate SHA-256 values in
the table, and both deterministic validators passed against the applied files.
Owner approval of one proposal did not authorize the other. These smoke files
still do not satisfy the full SvelteKit, route, component, adapter or automated-
test requirements above.

The maintainer then explicitly confirmed every `HP-PROT-003` ceiling and the
inert UTF-8 payload condition. The activated smoke slice was deliberately
narrower than the full proposition: one 32,119-byte metadata-and-UTF-8 JSON
matrix representing all 36 required cases, with 1,304 physical payload bytes,
zero archive entries, zero physical represented paths and no generated package
artifact. It performs no extraction, execution, scanning, network access,
upload or publication and defines no package schema or validator.

The installed `qwen3-coder:30b` primary produced complete case coverage but
also invented one dependency category, misclassified the valid-control
hypotheses and supplied incorrect byte counts. Codex corrected those critical
contract and evidence defects against the selected ADR, operations, licensing
and threat-model sources before proposal creation. The installed read-only
`gpt-oss:20b` challenger did not see the primary response; its suggestions for
real links, nested archives, partial extraction, algorithm variants and binary
cases were rejected as outside this smoke authority. No challenger prose was
copied into the candidate.

The deterministic validator rejected the empty seed and accepted the reviewed
candidate, including exact ceiling metadata, unique case coverage, recomputed
UTF-8 byte counts, zero physical archive/path construction, complete decision-
report partitioning and forbidden-content/control-character scans. Commissioned
Scribe read-back confirmed the exact 1,025-byte seed before proposal creation.
The first exact proposal expired without changing the seed. Fresh proposal
`proposal-e7d7f25541985f6b5558e387b2216b65`, exact proposal hash
`0499acec874b439d6907875d663d580b899c3bcb4d61cac7967b357a96b82f89`,
was then created for the unchanged candidate SHA-256
`ef9710c2a655a2556caf08338cfe051f4b09368e45c08b4354454112b7f2497b`.
The owner approved that exact hash and the proposal reached `succeeded`.
Commissioned read-back reported the same full-file hash and 32,119-byte length;
independent `K:` hashing matched, and the deterministic validator passed against
the applied file. This remains only the metadata-and-UTF-8 smoke slice: the full
generator, reproducibility, archive and extraction-boundary requirements remain
open.

## Further activation order

The completed single-file smoke slices do not satisfy or skip the fuller
implementation order below.

1. Run the micro-task bake-off for `HP-PROT-001` and `HP-PROT-002` in separate
   disposable copies, then activate their winning model projects independently.
2. Review both presentation handoffs without combining their source.
3. Activate `HP-PROT-003` only after its size and construction ceilings are
   written into the local work order and a person confirms that every planned
   artifact is inert.
4. Do not open an integration change until each prototype has its own completed
   handoff and the maintainer explicitly selects what, if anything, survives.

The prototypes may be discarded independently. Failure of one does not block
ordinary WyrmGrid work or authorize another to expand its scope.

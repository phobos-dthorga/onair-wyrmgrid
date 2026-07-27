# Jenkins operations

Jenkins is WyrmGrid's routine CI, snapshot, and release-candidate build system.
The root `Jenkinsfile` contains no credential binding and has no publication
authority. It may invoke the controller-configured ForgeAI service for bounded
advisory review. A separate trusted Pipeline reads `Jenkinsfile.release` and is
the only Jenkins job allowed to receive GitHub release authority.

The ForgeAI integration is an explicit exception to the general rule that
optional local-AI development tasks stay outside CI. It does not invoke
Hoardmind or `scripts/run-optional-ai-task.mjs`, and the exact-tag release
pipeline performs no model call. Jenkins sends no OnAir credential, raw
provider payload, personal flight data, database content, signing key, or
end-user device key to ForgeAI.

## Required Jenkins capabilities

The controller requires Declarative Pipeline, Git, GitHub Branch Source,
Credentials Binding, and
[ForgeAI Pipeline Intelligence](https://plugins.jenkins.io/forgeai-pipeline-intelligence/).
The pipelines use built-in checkout, preserved stash, archive, fingerprinting,
timeout, restart, and approval steps; they do not require Copy Artifact, HTML
Publisher, or a build-cache plugin. Keep ForgeAI on a reviewed version
compatible with the controller. Configure its OpenAI-compatible endpoint and
model centrally, store any endpoint token as a Jenkins Secret Text credential,
keep score-based failure disabled, and never place the endpoint token or
resolved secret in this repository or a build log.

Configure two agents:

- `linux`: Ubuntu or Debian with Node.js 22, npm 10 or newer, Rust 1.97 with
  rustfmt and Clippy, Python 3 available as `python`, `cargo-deny`, GitHub CLI,
  and the Tauri WebKit/AppIndicator/RSVG/AppImage/Debian prerequisites already
  installed. Builds do not run `sudo` or mutate system packages.
- `windows`: Node.js 22, npm 10 or newer, Rust 1.97 with rustfmt and Clippy,
  PowerShell 7, `cargo-deny`, Visual Studio Desktop development with C++,
  WebView2, and Strawberry Perl. Run the agent as an identity that can access
  that toolchain.

Set the controller executor count to zero and do not give the controller the
`linux` label. Start with one executor on each dedicated agent. Platform gates
are intentionally separate, sequential top-level stages so Jenkins can restart
at an individual failed stage without repeating completed gates. Increase agent
capacity rather than weakening application deadlines when a single cold
revision cannot finish within 60 minutes.

The preferred Proxmox worker is a dedicated unprivileged LXC container on VLAN
20 with the `linux` label, one executor, four CPU cores, and 8192 MiB of memory.
Keep its filesystem, service account, and Jenkins remote root separate from the
`ubuntu-agent` VM template used by Hoardmind and other AI work. Do not enable
nesting, device passthrough, privileged mode, or host filesystem mounts merely
to make a build pass. Rebuild the container from a reviewed base and toolchain
manifest when it drifts; do not make the Jenkins controller an eligible build
node as a fallback.

Give the Linux worker its own continuously renewed Teleport Machine ID and a
narrow role limited to the worker's required WyrmGrid-labelled resources. Do
not copy the TaurykDesktop, Codex, GitHub Actions, or another agent's identity
into the container. Jenkins administration and build-time remote access must
use Teleport; never add a static SSH key, an `authorized_keys` fallback, or
direct port-22 access.

The Windows helper uses `WYRMGRID_CARGO_TARGET_ROOT` when the node defines it,
then creates a bounded hash directory for each complete Jenkins job identity.
If the variable is absent, it uses the agent identity's local application-data
directory. A short dedicated value such as
`C:\JenkinsCache\WyrmGrid\cargo-target` is recommended.

## Multibranch organization job

Create a dedicated GitHub App named `WyrmGrid Jenkins CI`, install it only on
`phobos-dthorga/onair-wyrmgrid`, and store it as the Jenkins GitHub App
credential `wyrmgrid-github-ci`. Grant repository Contents read-only, Metadata
read-only, Pull requests read-only, and Commit statuses read/write. Configure
the credential to infer the accessible repository and restrict untrusted build
use to read-only repository contents. This status-only authority is not release
or repository-content publication authority. Never reuse or broaden the
Hoardmind generated-contribution App for Jenkins.

Configure the GitHub Organization Folder with that credential and
`Jenkinsfile` at the repository root. Discover branches with **Exclude branches
that are also filed as PRs** and discover origin pull requests with **Merging
the pull request with the current target branch revision** only. This produces
one authoritative synthetic-merge build for an origin pull request instead of
competing branch, head, and merge builds. Keep fork pull requests on their
restricted trust policy and outside ForgeAI.

The organization-folder merge-only discovery setting is part of the ForgeAI
trust boundary. Jenkins exposes an eligible origin merge build as `PR-<number>`
rather than appending `-merge`, so the pipeline recognizes it through
`CHANGE_ID` with no `CHANGE_FORK`. Do not enable origin pull-request head
discovery without also revising this eligibility contract.

Configure GitHub webhook delivery so pushes and pull-request updates re-index
the folder. Keep a periodic organization scan as recovery for missed webhook
events, not as the primary trigger. Prefer the dedicated GitHub App's
registration webhook. If GitHub cannot persist that registration-level hook,
configure the same endpoint as a repository webhook limited to `push` and
`pull_request` events, use JSON payloads, and keep SSL verification enabled.
After the first successful exact-head build, require its stable Jenkins status
context in the `main` branch protection rules instead of the former
automatically triggered GitHub Actions jobs. The single pipeline status
represents both deterministic platform lanes; it succeeds only when both do.

Every discovered revision runs Linux and Windows validation. A newer build of
the same branch cancels the older one. Only `main` and `codex/release-*` build
and retain unsigned snapshots:

Rust test cases run serially within each job. Jenkins may validate several
discovered revisions at once, and serial test execution prevents agent
contention from consuming the real three-second plugin startup handshake
without weakening that production deadline.

- Linux AppImage and Debian package;
- Windows per-user NSIS setup with a clean-install smoke test;
- platform-specific `BUILD-INFO.json` and `SHA256SUMS.txt`.

Artifacts are retained for 14 days or ten artifact-bearing builds, whichever
limit Jenkins reaches first. Build metadata contains the application version,
source ref, exact commit, Jenkins build number, supported platform identifiers,
and explicit unsigned/checksums-only declarations. It excludes usernames,
hostnames, absolute paths, credentials, and raw application data.

After deterministic validation and any eligible snapshots finish, ForgeAI runs
once for `main` and once for an origin pull request's merged-with-target
revision. Ordinary branches, duplicate pull-request head jobs, forked pull
requests, and the release pipeline skip it.

`scripts/prepare-forgeai-review.mjs` compares the built commit with its first
parent and writes one generated packet beneath ignored
`.jenkins/forgeai-input/`. The packet:

- includes commit subjects and patches for at most 40 changed implementation,
  pipeline, and dependency-manifest files;
- is limited to 60 KiB, apportions the patch budget across files, and records
  omitted scope;
- excludes fixture, snapshot, capture, recording, payload, dependency-cache,
  and build-output paths;
- contains only repository-relative paths and refuses common credential or
  private-key signatures, invalid UTF-8, unsafe control characters, and
  bidirectional overrides; and
- labels source, comments, commit text, and patches as untrusted advisory
  evidence.

The code, architecture, test-gap, commit, and vulnerability analyzers receive
that packet. ForgeAI's pipeline advisor independently reads the current root
`Jenkinsfile`, and its dependency analyzer independently reads the checked-out
dependency manifests supported by the plugin. Before invocation, the generator
enumerates those tracked special inputs, requires valid UTF-8, rejects the same
credential and control-text signatures, and caps their combined size at 100
KiB. The clean checkout occurs before dependency installation, so generated
`node_modules` or build manifests are absent.

The analyzers run in feature-first order: code review, architecture drift, test
gaps, commit intelligence, and pipeline advice, followed by vulnerability and
dependency risk. Release readiness is deliberately omitted. The self-contained
HTML report is archived as an ordinary build artifact. The reported score,
severity, suggestions, and vulnerability labels are untrusted model output;
they neither prove a gate passed nor establish release, compatibility, or
security readiness.

ForgeAI's current full-analysis step catches individual analyzer failures.
WyrmGrid therefore checks that all seven requested analyzers returned. A
timeout, packet refusal, plugin error, missing analyzer, or report-archive
failure marks only the ForgeAI stage unstable while preserving a successful
overall result. A reported seven-analyzer success must also produce at least one
regular file under `forgeai-reports/`; otherwise the advisory stage is
incomplete. Reports containing the plugin's `JSON parsing failed` fallback are
also incomplete: the fallback score and empty findings are not accepted as
valid analyzer evidence. Partial and malformed reports are still archived for
diagnosis. Because this advisory stage is last, it cannot prevent deterministic
validation or snapshot creation.

## Manual Jenkins AI Agent job

`Jenkinsfile.ai-agent` defines a separate, manually launched research and
implementation job. It is not a branch-discovery pipeline, does not replace
ForgeAI, and never runs in the exact-tag release job. The installed
[Jenkins AI Agent plugin](https://plugins.jenkins.io/ai-agent/) remains the
conversation, execution, and usage-statistics interface for both local
OpenCode and an explicitly selected hosted review.

The six modes are:

- `ASK`, for a cited question against the selected immutable revision;
- `DECISION_TRACE`, for following an architectural or operational decision
  through its sources;
- `CONSISTENCY_AUDIT`, for evidence-backed contradictions or drift;
- `ROADMAP_STATUS`, for implemented, remaining, and uncertain roadmap work;
- `PATCH`, for a small bounded correction; and
- `FEATURE`, for a larger but still explicitly scoped implementation.

Read-only modes archive Markdown and structured JSON with exact commit-bound
`path:line` citations and must leave no tracked diff. `PATCH` and `FEATURE`
require `ALLOWED_PATHS`, may create, modify, rename, or delete textual files in
those paths, and must select a test profile from
[`ci/ai-agent-policy.yml`](../../ci/ai-agent-policy.yml). Dependency
manifests, migrations, CI files, and security-policy files are not categorically
forbidden; their exact path must be explicitly included. Opaque binary changes,
generated binaries, root-wide scopes, wildcards, traversal, symlink escapes,
secrets, excess files, excess lines, and changes outside the declared scope are
rejected before publication.

The local model receives at most two bounded correction passes for an invalid
answer or missing/invalid diff. Jenkins then runs the selected checked-in test
profile outside the model and may return bounded, redacted failures for at most
two test-and-repair passes. A persistent failure releases the executor before
asking whether to archive the patch or open a clearly marked failing draft.
No response within 24 hours defaults to archive-only.

A passing change becomes one namespaced branch and one draft pull request.
Jenkins commits and publishes only the exact revalidated diff. The local model
does not receive GitHub credentials and cannot approve, merge, publish a
release, modify repository settings, or make a draft ready for review.

### Editable context tiers

The model-visible file limits are intentionally ordinary checked-in policy, not
hard-coded Jenkins parameters. Open
[`ci/ai-agent-policy.yml`](../../ci/ai-agent-policy.yml) and find:

```json
"context_limits": {
  "active_profile": "SMALL_FILES",
  "profiles": {
    "SMALL_FILES": { "...": "initial ceilings" },
    "MEDIUM_FILES": { "...": "next commissioning tier" },
    "LARGE_FILES_RESEARCH": { "...": "attended research tier" }
  }
}
```

Change only `active_profile` to promote every run to an already reviewed tier.
To tune a tier, edit its three nearby values:
`maximum_visible_file_bytes`, `maximum_visible_file_lines`, and
`maximum_visible_total_bytes`. The file uses JSON-compatible YAML so the
dependency-free Python policy runtime can parse it. Keep the syntax as strict
JSON.

Initial `SMALL_FILES` commissioning allows 32 KiB or 800 lines per file and
512 KiB across the complete sparse worktree. `MEDIUM_FILES` permits 64 KiB,
1,200 lines, and 1 MiB. `LARGE_FILES_RESEARCH` permits 128 KiB, 2,500 lines,
and 2 MiB. Promote only after reviewed `ASK`, `PATCH`, and `FEATURE` canaries at
the current tier. Oversized paths remain visible in the documentation inventory
with their path, size, line count, and hash, but their contents are not copied
into model context. An exact oversized write target fails before inference.

These file ceilings are separate from the local model's 4,096-token context and
1,200-token response limits. Repository Scholar and Scoped Builder are friendly
profile names; their current exact local model IDs and provisional selection
status remain visible in the policy.

### Worker and pinned tools

Run the job on the dedicated unprivileged VLAN 20 LXC with label `ai-agent` and
one executor. It may share that single research executor sequentially with the
Hoardmind commissioning job, but it must not share repository workspaces,
Gateway clients, GitHub Apps, or Jenkins credentials. Keep the controller at
zero executors. OpenCode sends inference to Hoardmind Gate; it does not require
GPU access inside the container.

The policy pins OpenCode `1.18.5` and OpenAI Codex CLI `0.145.0`. Every run
records a privacy-reduced toolchain report and fails before inference if the
worker reports another version. Upgrades require their own reviewed source
change and commissioning canary.

### Credentials and manual job creation

Provision three separately scoped Jenkins credentials:

- `wyrmgrid-ai-agent-gateway`: Secret Text for a concurrency-one Hoardmind Gate
  client named `jenkins-wyrmgrid-ai-agent`;
- `wyrmgrid-jenkins-ai-contributor`: GitHub App credential for the
  repository-restricted `WyrmGrid Jenkins AI Contributor` App; and
- `Codex-Jenkins-Tauryk-Gk-Io`: the existing global Secret Text OpenAI API key,
  used only when `OPENAI_AFTER_DRAFT_PR` is selected.

Install the contributor App only on `phobos-dthorga/onair-wyrmgrid`. Grant
Metadata read-only, Contents read/write, and Pull requests read/write. Grant no
Administration, Actions, Checks, Environments, Issues, Members, Secrets,
Workflows, merge-queue, or organization authority. A client secret is not
needed. Private-key generation, rotation, conversion, and Jenkins credential
entry are manual maintainer actions; never paste a key into a job parameter,
prompt, artifact, build log, or repository file.

Create a manually triggered Pipeline from SCM named `wyrmgrid-ai-agent`:

- repository: `https://github.com/phobos-dthorga/onair-wyrmgrid.git`;
- script path: `Jenkinsfile.ai-agent`;
- Jenkins credential for source checkout: the existing read-only
  `wyrmgrid-github-ci`;
- no SCM polling, webhook trigger, timer, or automatic upstream trigger; and
- branch: the commissioning source branch until the implementation is merged,
  then `main`.

The job validates that the local Gateway credential exists for every run, the
contributor credential exists before a change-making run, and the hosted key
exists only when hosted review is selected. Reports, conversations, policy
evidence, tests, and patches are retained for 30 days and at most 20 builds.

### Optional hosted review

`OPENAI_AFTER_DRAFT_PR` is per-run consent for hosted processing. Jenkins first
opens a passing draft PR, records an outbound manifest, and constructs a
separate packet directory containing only the exact diff, test results,
relevant excerpts from the declared documentation scope, and the repository
rules required to review that change. OpenAI Codex runs in that directory with
ephemeral state, no web search, `gpt-5.6-sol`, and `xhigh` reasoning.

The structured result is archived and posted as a non-approving `COMMENT`
review. Actionable findings inside the original scope may receive one local
repair pass. If the same tests pass, Jenkins pushes one follow-up commit and
runs one hosted verification review against the new exact SHA. It does not
continue beyond that cycle. A failed hosted repair is archived and discarded,
leaving the draft PR at its last passing commit.

### Reuse by another owned repository

Another owned repository adopts this pattern through its own manual Pipeline,
checked-in `ci/ai-agent-policy.yml`, documentation roots, test registry,
context tiers, Hoardmind Gate client, and repository-restricted contributor
App. Never add a second repository to the WyrmGrid App installation or reuse
the WyrmGrid Gateway secret. The same draft-only, no-merge lifecycle applies,
but model selection and file ceilings are commissioned independently against
that repository's actual corpus.

## Trusted release job

Create a separate locked Jenkins folder containing one Pipeline job sourced
from `main` at `Jenkinsfile.release`. Do not create this job beneath the GitHub
Organization Folder. Disable ad-hoc Pipeline reconfiguration for users who are
not release maintainers.

Do not add ForgeAI, another model, or an optional-AI task to this release job.
Release notes, semantic-version decisions, exact-tag acceptance, approval, and
publication remain deterministic or human-controlled.

Create a dedicated GitHub App installed only on
`phobos-dthorga/onair-wyrmgrid` with repository Contents read/write. Store it as
a Jenkins GitHub App credential named `wyrmgrid-github-release` at the locked
release-folder scope. Do not reuse the Hoardmind generated-contribution App:
that App has a different identity, policy, path boundary, and authority.

The release job accepts:

- `RELEASE_TAG`: an existing `vX.Y.Z` or supported prerelease tag;
- `EXCEPTION_REASON`: at least 20 non-whitespace characters explaining the
  exact tagged build.

The job never creates or moves a tag. It verifies the tag's four application
versions, installer identity, changelog entry, exact commit, and ancestry from
`origin/main`. It refuses to replace a published release. It repeats all Linux
and Windows gates, builds exact-tag packages, and runs the Windows clean-install
or nearest-release upgrade test.

After assembly, Jenkins archives and fingerprints the three packages,
`BUILD-INFO.json`, and `SHA256SUMS.txt`, then pauses for explicit human
approval. Only the narrowly scoped release-state query, previous-installer
download, and final publication commands receive the short-lived App token.
The publication commands create or update a draft prerelease; they never
publish it.

Jenkins releases currently provide checksums and honest build metadata, not
cryptographically signed provenance. GitHub Actions-native attestations cannot
describe a build performed on Jenkins. A future Sigstore or equivalent design
requires its own identity, key or OIDC, verification, recovery, and threat-model
decision. Do not label `BUILD-INFO.json` as an attestation.

## Manual GitHub fallback

The checked-in GitHub CI and security workflows are callable only by the release
workflow. The release workflow has only a manual trigger and accepts an existing
tag plus a meaningful exception reason. It builds Windows and Linux packages
and retains GitHub's native attestation path for an explicitly authorized
emergency rebuild.

Routine pushes, pull requests, schedules, and version tags do not start those
hosted workflows. This prevents GitHub Actions and Jenkins from racing to edit
the same draft release.

## Failure handling

- Jenkins preserves stashes for the five most recent builds. After correcting a
  transient agent or pipeline failure, use **Restart from Stage** on the failed
  top-level platform or assembly stage. Jenkins skips earlier completed stages
  and restores the policy/package inputs needed downstream. A new build caused
  by a new commit still runs every gate, and release publication still requires
  the explicit approval stage.
- A missing or offline agent leaves the associated stage queued until its
  timeout; it does not silently downgrade platform coverage.
- A failed validation prevents snapshot and release packaging.
- A failed or incomplete ForgeAI review leaves deterministic results and
  snapshots intact, marks the advisory stage unstable, and may be rerun after
  the model service recovers.
- A missing package, duplicate normalized filename, failed NSIS smoke test, or
  published release blocks publication.
- A failed draft upload leaves the Jenkins artifacts available for diagnosis.
  Do not hand-publish them or bypass the exact-tag job; repair the cause and run
  the same tag with a new documented reason.
- Final GitHub publication remains a separate human decision after installation,
  checksum, notes, licence, and offline-behaviour review.

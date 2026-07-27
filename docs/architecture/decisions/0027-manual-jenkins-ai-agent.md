# ADR-0027: Manual scoped Jenkins AI Agent

- Status: Accepted
- Date: 2026-07-27

## Context

WyrmGrid already has two distinct optional-AI paths. Maintainer-run local task
contracts can draft bounded advice or patches, and ForgeAI reviews trusted
Jenkins revisions after deterministic validation. Neither path provides an
interactive, manually launched research agent that can interrogate an immutable
repository revision, perform a scoped implementation, retry after deterministic
feedback, and publish its result for ordinary human review.

The first Hoardmind commissioning experiments showed that a local model does
not need frontier-model perfection to be useful, but it does need bounded
context and repeated opportunities to correct concrete failures. WyrmGrid is
the safer first change-making target because most tracked text files are small.
Large generated, lock, map, and presentation files remain poor initial context
for a local model.

## Decision

WyrmGrid adds a separate, manually launched Pipeline from
`Jenkinsfile.ai-agent`. It uses the Jenkins
[AI Agent plugin](https://plugins.jenkins.io/ai-agent/) as the execution,
conversation, and usage interface. ForgeAI retains its existing advisory
pull-request role and is not replaced.

The job supports four read-only modes:

- `ASK`;
- `DECISION_TRACE`;
- `CONSISTENCY_AUDIT`; and
- `ROADMAP_STATUS`.

It also supports two change-making modes:

- `PATCH`, for a small repair; and
- `FEATURE`, for a bounded multi-file implementation.

Every run resolves `main` or a full commit reachable from `main` to one
immutable commit. Read-only results require commit-bound `path:line` citations
and cannot produce a tracked diff. Change modes require explicit repository-
relative path scopes, select a checked-in test profile, enforce file and line
ceilings, and may create, edit, rename, or delete textual files within that
scope. Jenkins—not the model—runs registered tests and may return bounded,
redacted failures for at most two local repair passes.

The model receives a sparse worktree containing only eligible files. The
authoritative limits live in [`ci/ai-agent-policy.yml`](../../../ci/ai-agent-policy.yml)
under `context_limits`. Initial commissioning uses `SMALL_FILES`: no visible
file may exceed 32 KiB or 800 lines, and all model-visible files together may
not exceed 512 KiB. `MEDIUM_FILES` and `LARGE_FILES_RESEARCH` are checked-in
promotion tiers. An operator changes one `active_profile` value to promote all
runs, or edits the nearby numeric ceilings after reviewed canary evidence.
Oversized files remain inventory-visible but content-hidden. An exact oversized
change target fails before inference, and a model cannot modify any file hidden
by the active context profile.

OpenCode uses Hoardmind Gate through a dedicated WyrmGrid client with
concurrency one. Friendly profiles identify a local repository scholar and a
local scoped builder while retaining exact model IDs in the policy. Initial
selection is provisional until WyrmGrid-specific documentation and small-file
change canaries compare the installed candidates by answer quality.

A passing change is committed by Jenkins on a namespaced branch and opened as
one clearly labelled draft pull request. The model never receives GitHub
credentials. Jenkins uses the repository-restricted `WyrmGrid Jenkins AI
Contributor` App only after the complete diff and tests pass. The App can write
repository contents and pull requests but cannot approve, merge, administer,
publish a release, or alter repository settings.

If tests still fail after two repair passes, Jenkins releases the executor and
offers archive-only or a clearly failing draft pull request. No response within
24 hours defaults to archive-only.

An explicit `OPENAI_AFTER_DRAFT_PR` choice authorizes one hosted review. The
plugin runs OpenAI Codex in an isolated packet directory containing only the
exact diff, tests, selected documentation excerpts, and repository rules. It
posts a non-approving comment review. At most one in-scope local repair and one
hosted verification review may follow. The last passing draft commit is
preserved if that repair fails.

Each adopting repository owns its own job, policy, documentation roots, tests,
Hoardmind Gate client, and repository-restricted GitHub App. This repository
does not grant the WyrmGrid identities access to another repository and does
not modify another repository during commissioning.

## Consequences

Maintainers gain one visible Jenkins conversation for documentation research,
decision tracing, consistency work, roadmap reconciliation, small repairs, and
bounded features. Concrete validation failures can improve a merely workable
local model through repeated passes without turning the model into a release or
merge authority.

The conservative file limits deliberately exclude some high-value large files.
That is a feature of the first research tier, not a claim that those files can
never be supported. Promotion is a small, reviewable policy change after
measured canaries.

Sparse visibility, path validation, registered tests, and draft-only
publication reduce risk but do not make local output correct. Human review,
ordinary protected CI, and all existing compatibility, migration, security,
and release decisions remain required.

The workflow adds a dedicated one-executor `ai-agent` worker and three
credentials. The Jenkins controller remains at zero executors. GitHub App
private-key generation, rotation, conversion, and Jenkins credential entry are
manual maintainer operations; no key belongs in repository content, prompts,
artifacts, or logs.

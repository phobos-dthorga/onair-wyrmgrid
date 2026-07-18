# Optional local-AI development tasks

These repository helpers are optional, local development tools. They are not
part of the WyrmGrid application, plugin platform, build, test suite, CI, or
release authority. Hoardmind is the maintainer's private profile; another user
may select their own loopback Ollama or unauthenticated OpenAI-compatible local
server. Manual work remains equally supported.

The generic runner is `scripts/run-optional-ai-task.mjs`. Every invocation
selects one built-in, versioned task contract, one explicit packet, and one
private local profile. It refuses CI, non-loopback endpoints, missing approval,
secret-like packet content, oversized packets, missing packet headings, model
substitution, missing exact token usage, and malformed output headings.

## Supported tasks

| Task ID                   | Purpose                                                                                       | npm command                       | Handoff template                                 |
| ------------------------- | --------------------------------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------ |
| `release-curation-v1`     | Draft the four required changelog categories from reviewed release evidence                   | `npm run ai:release-curation`     | [Template](templates/release-curation-v1.md)     |
| `change-impact-v1`        | Build an affected-component, test, documentation, compatibility-flag, and changelog dossier   | `npm run ai:change-impact`        | [Template](templates/change-impact-v1.md)        |
| `test-matrix-v1`          | Draft success, boundary, failure, unavailable-data, and regression cases for an approved rule | `npm run ai:test-matrix`          | [Template](templates/test-matrix-v1.md)          |
| `docs-sync-v1`            | Find evidence-backed documentation candidates and draft narrow replacements                   | `npm run ai:docs-sync`            | [Template](templates/docs-sync-v1.md)            |
| `fixture-variants-v1`     | Draft sanitized valid and invalid fixture variants from an approved contract                  | `npm run ai:fixture-variants`     | [Template](templates/fixture-variants-v1.md)     |
| `implementation-patch-v1` | Draft one bounded textual patch for explicitly allowed paths                                  | `npm run ai:implementation-patch` | [Template](templates/implementation-patch-v1.md) |
| `failure-triage-v1`       | Cluster sanitized local failures and propose narrow, non-destructive checks                   | `npm run ai:failure-triage`       | [Template](templates/failure-triage-v1.md)       |

The task prompts under [`tasks/`](tasks/) are public review boundaries. The
[base system prompt](base-system-prompt-v1.md) supplies the common no-tools,
review-only, untrusted-evidence contract. Profiles select only the local runtime
and base prompt; they do not change a task's required packet or output shape.

## Running a task

Copy one of the
[example profiles](../../examples/optional-ai/) into `.wyrmgrid-local/`. Change
the exact local model ID and set `system_prompt_file` to
`../docs/optional-ai/base-system-prompt-v1.md`. Copy the chosen handoff template
to a temporary file, replace every placeholder with bounded sanitized evidence,
and run its npm command. The runner rejects an untouched template.

```powershell
$profilePath = '.wyrmgrid-local\optional-ai-profile.json'
$packetPath = Join-Path $env:TEMP 'wyrmgrid-optional-ai-handoff.md'
$reportRoot = Join-Path $env:TEMP 'wyrmgrid-optional-ai-task'

npm run ai:change-impact -- `
  --packet $packetPath `
  --profile $profilePath `
  --output $reportRoot `
  --approve-once
```

The approval covers only that invocation. Use `npm run ai:task -- --task
<task-id> ...` when scripting against the generic entry point.

Each run writes a task-labelled draft, schema-versioned privacy-reduced metrics,
and an efficiency report. Metrics contain task/profile identity, exact local
tokens, available timing and resource observations, and explicit unavailable
states. They never contain the packet, prompts, or response text. Local tokens
are not relabelled as hosted tokens saved without separate hosted usage data.
The durable structures are the
[profile schema](../../schemas/optional-ai-task-profile-v1.schema.json) and
[metrics schema](../../schemas/optional-ai-task-metrics-v1.schema.json).

## Sequential use without autonomous chaining

For a substantial reviewed change, use the tasks in this order when applicable:

1. create and reconcile a `change-impact-v1` dossier;
2. prepare separate `test-matrix-v1`, `docs-sync-v1`, and
   `fixture-variants-v1` packets from confirmed parts of that dossier;
3. optionally draft one `implementation-patch-v1` result from an independently
   reviewed packet and extract it without feeding another model response into it;
4. run deterministic validators, tests, formatting, and builds;
5. use `failure-triage-v1` only for sanitized failures;
6. curate release notes after the implementation and compatibility decisions are
   complete.

The runner never automatically feeds one model response into another. A person
or coordinating reviewer must reconcile every draft against repository evidence
before selecting facts for the next packet. This prevents an early
hallucination from silently becoming downstream authority.

## Optional generated-contribution attribution

A maintainer may publish a wholly model-generated patch through a dedicated,
least-privileged GitHub App so its commit and branch have a separate bot
identity. After discarding the App token, the human maintainer opens the draft
PR. The assistant never receives the App key or token, and the App has no Pull
requests, merge, or release authority. The two-phase, hash-bound broker and
exact registration permissions are documented in the
[GitHub attribution guide](github-app-attribution.md). Human or materially
rewritten work remains human-authored and may use an `Assisted-by` note instead.

## Non-delegable boundaries

These tasks do not decide business rules, security, privacy, legal meaning,
protocol/schema compatibility, migration policy, live provider behaviour,
release versions, tags, or publication. They do not replace deterministic
formatters, linters, compilers, test runners, schema validators, coverage tools,
dependency audits, or release gates. Never include credentials, raw provider
payloads, databases, personal paths, crash dumps, or identifying operational
history in a packet.

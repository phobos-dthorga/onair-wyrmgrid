# Codebase structure audit — July 2026

## Purpose and scope

This audit reviewed the complete maintained repository at `03a5fea`, including
the stacked Atlas weather work. It covered Rust domain, application, provider,
storage and protocol crates; the Tauri adapter; Svelte and TypeScript
presentation; first-party plugins and simulator providers; SQL migrations;
schemas and fixtures; development scripts; and project documentation.

The review looked for:

- constructed, duplicated, misplaced, or unvalidated identifiers;
- catalogue keys that could drift from their consumers;
- user-facing text bypassing the localization boundary;
- domain mutation, validation, or orchestration in Svelte components;
- business orchestration in Tauri commands;
- duplicated policy values and unexplained numeric limits; and
- violations of provider, plugin, migration, protocol, credential, provenance,
  or release boundaries.

Generated binaries, dependency trees, ignored local reports, credentials, raw
provider payloads, and personal data were excluded. A bounded optional-local-AI
change-impact dossier was used to challenge the affected-component and test
list. It was treated as untrusted review input: its supported test suggestions
were retained, while its unsupported suggestion to edit optional-AI governance
was rejected.

## Implemented findings

### Catalogue-key ownership

Translated Svelte code now receives a `TranslationKey` derived from the
canonical `en-AU` catalogue. Explicit typed maps replace constructed keys for:

- Settings unit presets and weather-profile descriptions;
- simulator connection states, detail codes, ritual steps, recording states,
  and recording events;
- authorization capability and grant-lifetime labels;
- Dispatch comparison titles and explanations; and
- the localized OnAir rate-limit operation error.

Unknown provider or application codes retain their controlled fallback text;
they are never reinterpreted as catalogue keys. The new localization audit
rejects unknown literal keys, template-constructed Svelte keys, and version
drift between the source catalogue, Rust validator, schema, and fixture.

Atlas renderer status, fallback, adaptive-quality, accessibility label, and
source-boundary text moved from direct English into source catalogue version 12. The selection of those messages moved into a tested weather presentation
module rather than remaining nested in Svelte markup.

### Desktop command identifiers

The desktop command audit compares every literal frontend invocation with the
Tauri handler registry and rejects direct Tauri invocation outside the shared
desktop client. At the audited state, all 62 frontend command names match all
62 registered handlers.

This guard checks identifier ownership; it does not prove that a command is
architecturally thin or behaviourally correct.

## Boundary results

### Svelte presentation

No reviewed Svelte component directly mutates Rust domain or persisted state.
Mutating actions call feature clients and application commands; settings edit a
local draft and delegate save; destructive recording and flash choices remain
presentation-level confirmation prompts. Filtering, sorting, formatting, map
composition, renderer fallback, responsive interaction, and browser-preview
state are presentation concerns.

Two composition surfaces have nevertheless exceeded a comfortable complexity
budget:

- `+page.svelte` combines application initialization, asynchronous feature
  orchestration, workspace state, dialogs, Atlas inspectors, and broad
  presentation labels;
- `AtlasMap.svelte` combines MapLibre source/layer registration, feature
  conversion, radar management, selection, animation, and detailed-renderer
  lifecycle.

These are maintainability risks, not evidence that current domain decisions
live in the UI. Future slices should extract cohesive presentation controllers
without moving browser or renderer concerns into Rust.

### Tauri adapter

Most commands delegate directly to an application service or adapt an
OS-specific operation such as credential zeroization, a blocking file task, or
sidecar supervision. The Dispatch command group is the principal exception:
`dispatch_status`, flight-operation start/revision, job selection, SimBrief
import, weather refresh, and plan clearing coordinate several application
services in the Tauri adapter.

That orchestration belongs behind an application-owned facade. Moving it is a
separate behavioural refactor because it spans dispatch, account preferences,
flight-operation availability, and simulator-recording plan context. It should
receive application-level success, boundary, failure, and unavailable-data
tests before the Tauri commands are reduced to delegation.

### Localization coverage

Localization remains intentionally incremental. Settings and other established
catalogue-backed surfaces are protected by typed keys, but older workspaces
still contain direct English interface copy. This is not a misplaced-key defect:
those strings do not yet have catalogue entries. Bulk migration was rejected
for this cleanup because it would enlarge the language-pack compatibility
change and make semantic review of hundreds of messages unreliable.

Future localization slices should migrate one cohesive surface at a time,
including accessible names, formatting policy, source-catalogue versioning,
and pseudo-locale or RTL verification.

### Constants and identifiers

Physical conversions, renderer budgets, adaptive-quality thresholds, protocol
versions, manifest limits, retention limits, storage resource kinds, and
security limits are generally named in focused modules. Local CSS geometry and
one-use MapLibre expressions were not promoted into global constants merely for
being numeric.

Atlas source and layer identifiers are named, but remain concentrated inside
`AtlasMap.svelte`. A declarative Atlas layer registry is justified when the map
controller is extracted; performing only a mechanical rename now would add an
abstraction without reducing the component's lifecycle complexity.

### Critical boundaries

The audit found no change required to OnAir's read-only boundary, credential
handling, plugin capability policy, raw-provider translation, protocol
versions, database layout, released migrations, legal meaning, installer
identity, release policy, or optional-AI governance. No existing migration was
edited and no provider support claim was added.

Source catalogue version 12 is an intentional compatibility advance: community
packs must review the added ordinary Atlas messages and update their declared
source version. The language-pack schema version remains 1.

## Follow-up order

1. Introduce an application-owned Dispatch facade and reduce its Tauri commands
   to adapter delegation.
2. Extract an Atlas presentation controller for renderer lifecycle and a
   declarative source/layer registry.
3. Split `+page.svelte` by feature orchestration and workspace presentation,
   preserving application-service ownership of business decisions.
4. Continue localization surface by surface, starting with accessible labels
   and the most frequently used Atlas and Dispatch controls.

The deterministic identifier audits should remain inexpensive routine gates.
They complement formatting, type checking, tests, builds, Clippy, and dependency
audits; they do not replace any of them.

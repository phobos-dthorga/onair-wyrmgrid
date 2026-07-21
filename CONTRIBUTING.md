# Contributing to OnAir WyrmGrid

Thank you for helping build WyrmGrid. The project welcomes core, documentation,
design, localization, testing, and plugin contributions.

## Before starting

For substantial changes, open an issue or discussion first. Plugin API, storage
schema, security, and domain-model changes have long-lived compatibility costs
and need an explicit design decision.

Good first contributions include documentation corrections, accessible UI
improvements, test fixtures with all personal data removed, and small isolated
plugin examples.

A contribution described as a plugin or provider must preserve an external
artifact boundary. It may add a supported script, executable, native library,
or future runtime payload, but it must not require community users to rebuild
or relink the WyrmGrid desktop. A first-party artifact may be included in an
official installer while remaining independently packageable and installable.
Ordinary plugin authors can create the current version-one package with
`npm run plugin:package -- --source <directory> --output <file.wyrmplugin>`;
see the [plugin platform guide](docs/plugins/overview.md) for inventory,
runtime, trust, and installation details.
Simulator provider authors can package an existing `provider.json` and its
declared executable with
`npm run provider:package -- --source <directory> --output <file.wyrmprovider>`;
see the
[simulator provider guide](docs/integrations/simulator-provider-authoring.md)
for Bridge compatibility, native-code trust, and lifecycle requirements.

## Local checks

```powershell
npm run format:check
cargo clippy --workspace --exclude wyrmgrid-desktop --all-targets -- -D warnings
cargo test --workspace --exclude wyrmgrid-desktop
npm ci
npm run check
npm run build
```

Run `npm run format` to repair both Rust and frontend/document formatting
deterministically. Rust output is always normalized to the repository's LF
newline policy, including on Windows, so this routine repair does not require an
AI assistant.

Run `cargo check -p wyrmgrid-desktop` on a machine with Tauri's native
prerequisites installed.

## Pull requests

- Keep each pull request focused and explain the user or developer impact.
- Include tests for new behavior and update the relevant documentation.
- Never include credentials, company identifiers, personal flight history, or
  raw API payloads that have not been carefully sanitized.
- Call out breaking changes to plugin, database, event, or domain contracts.
- Use conventional commit-style subjects where practical: `feat:`, `fix:`,
  `docs:`, `refactor:`, `test:`, `build:`, or `chore:`.

## AI is optional

No AI assistant is required to build, test, document, contribute to, or release
WyrmGrid. Hoardmind is a private maintainer tool and is not a WyrmGrid service,
dependency, contributor requirement, or privileged source of project truth.
Human-authored work follows exactly the same review and quality gates.

Contributors who independently choose an AI assistant remain responsible for
verifying its output and for keeping credentials, personal data, raw provider
payloads, and other private material out of prompts and contributions. The
repository's optional development-task runner is local-only, profile-driven,
review-only, and limited to built-in versioned contracts; using it is never a
condition of contribution.

A wholly assistant-generated textual patch may use the optional
[GitHub attribution workflow](docs/optional-ai/github-app-attribution.md). It
creates one hash-bound bot commit and branch through a dedicated GitHub App,
then opens a human-maintainer draft PR with protected paths and review enforced.
Materially human-written or rewritten changes remain human-authored and may
record narrow assistance with `Assisted-by:` instead. No contributor needs the
App or an AI assistant.

By contributing, you agree that your contribution is licensed under this
repository's MIT License.

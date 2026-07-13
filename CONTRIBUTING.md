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

## Local checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --exclude wyrmgrid-desktop --all-targets -- -D warnings
cargo test --workspace --exclude wyrmgrid-desktop
npm ci
npm run check
npm run build
```

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

By contributing, you agree that your contribution is licensed under this
repository's MIT License.

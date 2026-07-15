# OnAir WyrmGrid contributor instructions

## Product boundaries

- Treat the public OnAir API as read-only unless current official documentation
  explicitly establishes a supported write operation.
- Never log, serialize, expose to plugins, or commit an OnAir API key.
- Raw OnAir JSON belongs in `wyrmgrid-onair-api`; translate it into stable
  WyrmGrid domain models before other modules consume it.
- Preserve the distinction between raw facts, external facts, calculations, and
  recommendations using provenance metadata.
- Community plugins are out-of-process. Do not introduce public Rust, C++, Qt,
  or operating-system ABI coupling.
- Plugin permissions are deny-by-default and capability-oriented.

## Architecture

- Keep UI code presentational. Business rules belong in Rust application or
  domain services, never in Svelte event handlers.
- Keep Tauri commands thin and delegate to `wyrmgrid-application`.
- Keep SQLite migrations append-only after release. Never edit an already
  shipped migration; add a new numbered migration.
- Prefer a few cohesive crates over premature crate fragmentation.
- Reduce duplication and magic strings immediately when the shared abstraction
  is clear, but do not generalize hypothetical requirements.
- Native simulator integrations are separate sidecars and must degrade safely
  when MSFS, SimConnect, or FSUIPC is absent.

## Quality gates

- Rust: formatting, Clippy with warnings denied, unit tests, and dependency audit.
- Frontend: Svelte type checking, production build, and formatting.
- Keep test implementations physically separate from production modules. Rust
  production files may contain only a `#[cfg(test)]` path hook; put unit-test
  bodies in `src/tests/` and black-box integration tests in `tests/`. Keep
  frontend tests in dedicated `*.test.ts` files.
- Every bug fix needs a regression test at the lowest layer that can reproduce
  it. New business rules need boundary, failure, and unavailable-data cases as
  well as the successful path.
- Protocol changes require fixtures, validation tests, documentation, and an
  explicit compatibility decision.
- Security-sensitive changes require corresponding threat-model updates.
- Do not claim live OnAir behavior without a sanitized captured response or an
  authenticated integration test performed outside the repository.

## Releases

- Use semantic versioning for the application and separately version the plugin
  protocol, schema, and database migrations.
- CI produces release artifacts. Do not hand-assemble published binaries.
- Routine commits and pull requests compile-check the desktop application but
  do not assemble installers. Every intentional semantic-version release tag
  (`vX.Y.Z` or a supported prerelease) builds installers after the reusable CI
  and security gates pass. A manual rebuild must target an existing tag and
  record a concrete reason.
- Keep early releases marked as prereleases until update signing and platform
  signing policies are complete.
- Preserve the Windows installer's product name, application identifier, and
  per-user scope. Any intentional identity change requires an explicit migration
  design; routine setup upgrades must preserve application data and its
  device-local encryption key.

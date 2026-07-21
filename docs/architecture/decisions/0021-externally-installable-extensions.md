# ADR-0021: Externally installable plugins and providers

- Status: Accepted
- Date: 2026-07-21

## Context

WyrmGrid is intended to be a community platform. Its existing language-neutral
process boundaries allow external implementations, and ordinary Python plugins
are discovered from an application-data directory. Some first-party artifacts
are nevertheless still materialized from compile-time inputs or staged only as
part of the application build. Simulator and audio providers also have separate
protocols but do not yet share a complete end-user installation lifecycle.

If a feature called a plugin can be added only while compiling WyrmGrid, the
community cannot distribute it independently, users cannot manage it without a
new application build, and first-party code gains an architectural path that a
third party cannot use. That conflicts with the product's community focus.

Different integrations legitimately require different payloads. An ordinary
extension may be a script, a native provider may be an executable, and a
simulator may require a library loaded into the simulator's process. Requiring
one implementation language or file extension would be as restrictive as
requiring compile-time integration.

## Decision

Every component presented as a WyrmGrid plugin or capability provider has an
external, independently installable artifact boundary.

- Installing, replacing, disabling, updating, rolling back, or removing an
  extension must not require rebuilding or relinking the WyrmGrid desktop.
- First-party and community extensions use the same public protocol,
  permissions, compatibility, validation, and lifecycle rules for their package
  kind. First-party status may affect repository curation, not runtime
  privilege.
- An official application installer may include or seed compatible first-party
  packages for convenience. The same packages remain independently
  distributable, and WyrmGrid stores and runs them outside its executable.
- Package envelopes and payload formats are separate concerns. Supported
  payloads may include scripts, standalone executables, platform-native
  libraries required by another host, data assets, or future runtimes. Each
  package kind declares its manifest, runtime, entry point, platform and host
  compatibility, capabilities, network origins, dependencies, and content
  inventory.
- No community native library links into or is dynamically loaded by the
  WyrmGrid desktop process. If a simulator requires an in-process module, that
  module is installed into the simulator-facing boundary and communicates with
  a separately supervised provider over a versioned protocol.
- A new file format or runtime becomes executable only after an explicit host
  adapter, validation rules, fixtures, compatibility decision, and security
  review exist. Unknown artifacts remain inert.
- Manual installation of a locally obtained package is a core offline
  capability. It requires explicit user action and local validation but does
  not require Aerie, an account, or network availability.
- Aerie is an optional distribution layer for discovery, publisher identity,
  repository signatures, moderation, revocation information, and convenient
  updates. It must not become the authority required to use an already
  installed or locally verified package.
- A feature that must remain inside the desktop build is a core feature, not a
  plugin. Documentation and interface language must preserve that distinction.

The common lifecycle is discover, stage, validate, present trust and requested
capabilities, obtain approval, activate atomically, supervise, disable or stop,
update with renewed review where scope changes, roll back, and remove. Exact
package schemas and trust mechanisms remain separately versioned contracts.

## Migration direction

The current implementation moves toward this invariant in bounded steps:

1. stop treating compile-time materialization or Tauri external-binary staging
   as the authoritative installed form;
2. define package-kind manifests and canonical local installation roots for
   ordinary plugins, simulator providers, and audio providers;
3. implement Rust-owned staged install, validation, activation, provenance,
   rollback, disable, and removal transactions with a presentational Forge UI;
4. make official first-party artifacts use those transactions and remain
   separately downloadable or copy-installable;
5. add conformance fixtures and platform matrices for each supported payload
   type; and
6. add optional signed Aerie distribution only after local installation is
   complete and its security, legal, moderation, and recovery gates are met.

Until those steps are implemented, existing bundled and developer-preview
paths remain accurately labelled as current limitations and must not be cited
as the final plugin model.

The ordinary-plugin and simulator-provider portions are now implemented by
package schema version 1 in
[ADR-0022](0022-ordinary-plugin-package-format-v1.md) and
[ADR-0023](0023-simulator-provider-package-format-v1.md), including
deterministic first-party package artifacts. Audio provider packaging and
lifecycle are implemented by [ADR-0024](0024-audio-provider-package-format-v1.md)
with a deterministic synthetic reference artifact. Optional Aerie distribution
remains migration work.

## Consequences

Community authors can choose an appropriate supported implementation format and
ship on their own cadence. Users can compose their installation without waiting
for a WyrmGrid release, and first-party extensions prove the same public
boundaries that community extensions use.

WyrmGrid must maintain package compatibility, installation transactions,
provenance, permission review, safe failure, and platform-specific isolation
for more than one artifact kind. External delivery does not make native code
safe, so executable packages still require strong warnings and progressively
stronger integrity, isolation, signing, update, and revocation controls.

This decision extends
[ADR-0002](0002-out-of-process-plugins.md) and
[ADR-0011](0011-core-simulator-capability-provider-sidecars.md). It supersedes
ADR-0011 only where that record left community provider installation outside
the product target: local external provider installation is now required, while
broad recommendation and Aerie distribution still wait for their stated
hardening gates. Its process, protocol, capability, provenance, and
simulator-write boundaries remain accepted.

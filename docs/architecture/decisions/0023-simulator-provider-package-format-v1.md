# ADR-0023: Simulator provider package format version 1

- Status: Accepted
- Date: 2026-07-21

## Context

WyrmGrid Bridge already isolates native simulator integrations in supervised
sidecars, but the first SimConnect provider was staged only while building the
desktop installer. Community authors could implement the public protocol but
users had no supported way to install their executable without rebuilding
WyrmGrid. ADR-0021 requires the same independently installable artifact
boundary for first-party and community providers.

A simulator provider is more privileged than an ordinary plugin: it is native
executable code with the ambient rights of the user and may communicate with a
simulator SDK. Its package therefore needs its own kind and trust presentation,
while reusing the hardened archive inventory and managed-version lifecycle
where their semantics are genuinely the same.

## Decision

Simulator provider package schema version 1 uses a ZIP envelope with the
`.wyrmprovider` extension and media type
`application/vnd.wyrmgrid.simulator-provider-package+zip`.

The root `wyrmgrid-package.json` conforms to
`schemas/simulator-provider-package-manifest-v1.schema.json` and declares:

- package schema version `1` and kind `simulator_provider`;
- the same reverse-domain identifier and three-part semantic version as the
  enclosed provider manifest;
- the fixed provider manifest path `provider.json`; and
- the exact byte length and lowercase SHA-256 digest of every payload file.

The existing provider manifest remains authoritative for the provider name,
author, entry point, supported platform and architecture, simulator families,
Bridge protocol version, and capabilities. Its entry point must appear in the
package inventory. The package and provider identities and versions must match.
At launch, the provider's Bridge hello must still match that manifest exactly.

Provider packages inherit the version-one archive limits, canonical paths,
compression restrictions, exact-inventory validation, traversal and link
rejection, immutable `(kind, id, version)` identity, staged extraction,
versioned managed storage, interrupted-removal recovery, provenance, disable,
single-step rollback, and first-party seeding policy established for ordinary
packages in ADR-0021. On Unix hosts, managed extraction grants owner-only
execution to the declared entry point; no other payload becomes executable by
the installer.

Installation is an explicit two-step operation: WyrmGrid first shows the
identity, author, version, supported platforms and simulators, Bridge version,
capabilities, file count, expanded size, and full archive digest, then requires
separate confirmation. Installation does not launch the provider, select it, enable
auto-start, start recording, or grant a new Bridge capability. Package schema
version 1 has no publisher signature; `publisher_verified` is always false and
the interface identifies the payload as unverified native executable code.

Enabled managed providers are resolved into host-owned absolute paths and then
passed to the existing Bridge supervisor. Disabled providers disappear from
discovery. A running, starting, or stopping provider must be stopped before it
can be disabled, updated, rolled back, or removed. Disabling the selected
provider clears automatic start, and removal also clears the saved selection
before deleting package state, so enabling or reinstalling the same ID cannot
resurrect prior launch authority. An absent simulator, unsupported host
platform, missing executable, or failed provider remains a visible unavailable
state and must not crash WyrmGrid or trigger an automatic fallback.

The MSFS 2024 SimConnect provider is the first reference artifact. Its package
is produced deterministically as
`assets/provider-packages/msfs2024-simconnect.wyrmprovider`, is independently
distributable, and is optionally seeded from the desktop installer through the
same managed installation service used for a local community file. The desktop
no longer treats a Tauri external-binary declaration or compile-time manifest
as the installed provider.

## Compatibility

Provider package schema version, provider manifest schema version, Bridge
protocol version, and application semantic version are independent contracts.
This decision does not change Bridge protocol version 1 or provider manifest
schema version 1. Unknown package schemas, kinds, provider manifest schemas,
platforms, Bridge versions, compression methods, or capabilities remain inert.

A package may be inspected and stored on a host that its manifest does not
support, but it remains unavailable there. The current reference artifact is
Windows x86-64 only. A future provider for Linux or macOS uses the same package
kind when its existing provider manifest can express the target; a new payload
shape, simulator-loaded native module, dependency contract, or manifest field
requires an explicit compatibility and security decision.

An official seed may automatically advance only an active, older first-party
version. It never downgrades a package, takes control of a local-file
installation, or silently enables a package the user disabled. Local offline
installation does not require Aerie or network access.

## Consequences

Simulator providers can now be installed, replaced, disabled, updated, rolled
back, and removed without rebuilding or relinking WyrmGrid. First-party
SimConnect proves the same public delivery boundary available to a future
community FSUIPC or X-Plane provider.

Exact archive validation proves content consistency, not publisher identity or
code safety. A provider still runs with ambient user rights; version 1 has no
signature, revocation, authenticated update, pre-launch re-hash, operating-
system sandbox, or CPU and memory quota. Recommendation of unreviewed native
providers remains gated on that hardening even though deliberate local
installation is supported.

This decision implements the simulator-provider portion of
[ADR-0021](0021-externally-installable-extensions.md), reuses the bounded
envelope rules in [ADR-0022](0022-ordinary-plugin-package-format-v1.md), and
preserves the out-of-process Bridge boundary in
[ADR-0011](0011-core-simulator-capability-provider-sidecars.md).

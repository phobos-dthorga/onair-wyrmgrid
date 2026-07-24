# ADR-0026: Extension Developer Kit version 1

- Status: Accepted
- Date: 2026-07-24

## Context

WyrmGrid now has four independently installable extension artifacts and public,
versioned process protocols. The repository had separate scaffolding and
packaging scripts, but community authors still needed a WyrmGrid source checkout
to use them. Those scripts did not provide one consistent validation result,
independent archive reinspection, reproducibility proof, runtime handshake
test, or portable schema bundle.

That repository coupling conflicts with the same community boundary the
external package formats were created to provide. An authoring tool also sits
on a security-sensitive boundary: it handles untrusted paths and archives and,
when asked to test a runtime, deliberately executes extension code.

## Decision

Extension Developer Kit version 1 is a separately versioned, dependency-free
Node.js package named `@wyrmgrid/extension-developer-kit`. It exposes the
`wyrmgrid-extension` command and can be installed and used without a WyrmGrid
source checkout.

The command supports:

- `new`, which produces no-overwrite, deny-by-default starting trees for
  ordinary plugins, simulator providers, audio capture providers, and audio
  codec providers;
- `validate`, which validates a source manifest or independently parses and
  reinspects a completed extension archive;
- `package`, which produces each public extension format through one
  authoritative deterministic implementation;
- `test`, which validates, builds twice, requires byte-identical archives,
  reinspects the result, and performs a bounded protocol startup/shutdown
  handshake unless runtime testing is explicitly skipped; and
- `schemas`, which lists or copies the exact public schemas and SHA-256 schema
  catalogue bundled with that EDK release.

The EDK owns the authoritative JavaScript scaffolder and package builder.
Existing repository commands remain thin compatibility wrappers so first-party
package preparation and community package creation cannot silently diverge.

Compatibility report schema version 1 is a JSON artifact independent from the
EDK package version. It contains the tool and contract versions, operation,
basename-only target, extension identity, bounded check results, stable issue
codes, and archive digest. It excludes absolute source paths, extension
payloads, environment variables, credentials, and process output.

Archive inspection does not trust the package builder. It independently checks
ZIP framing, accepted compression, paths, links, collisions, file and expanded
sizes, CRC-32, exact declared inventory, SHA-256 payload digests, identity,
compatibility versions, and entry points. JSON manifests must be valid UTF-8.
Packaging always excludes its current output, common version-control metadata,
and the `.wyrmignore` control file. `.wyrmignore` accepts only exact safe
project-relative file paths and directory prefixes; it has no glob, negation,
absolute-path, or traversal syntax. Scaffolds exclude their distribution
directory, build caches, virtual environment, dependency tree, and `.env`.

Runtime conformance launches the author-selected extension with only a small
allowlist of process-discovery, temporary-directory, and locale environment
variables. The host sends no OnAir or provider credential, storage path,
database key, media key, simulator mutation authority, device permission, or
network grant. Control frames, binary bodies, buffered standard output, startup,
and shutdown are bounded. Standard error is drained but never copied into a
report. Launch failure, malformed framing, identity disagreement, non-monotonic
sequence, timeout, crash, or non-zero exit fails the report.

This process isolation is not an operating-system sandbox. The test command
must say that it executes the extension with the current user's ambient
operating-system rights. `--skip-runtime` is an explicit cross-compilation or
incomplete-scaffold escape hatch and remains visibly `skipped` in the report.

The npm package includes its license, schemas, schema catalogue, command, and
runtime implementation. Test-only fixtures and repository scripts are excluded.
Publication uses npm provenance when a separately authorized release is made.

## Compatibility

EDK version 1 supports ordinary plugin manifest/API version 1, simulator
provider manifest/Bridge version 1, Audio Capture Provider manifest/protocol
version 2, Audio Codec Provider manifest/protocol version 1, and all four
package-schema version 1 variants.

This decision adds EDK version `1.0.0` and compatibility-report schema version

1. It does not change the WyrmGrid application semantic version, any extension
   package, manifest, or process-protocol version, the application database, or
   the source localisation catalogue. Future EDK releases may add checks without
   changing an extension protocol, but a breaking command/report contract requires
   the corresponding EDK or report-schema compatibility decision.

An EDK report is evidence about a specific tool invocation, not a signature or
WyrmGrid acceptance receipt. WyrmGrid continues to validate every installed
package itself and does not trust an accompanying report.

## Consequences

Community authors can create and test WyrmGrid artifacts through one documented
tool without cloning or compiling WyrmGrid. First-party builds exercise the
same packager. Deterministic double-builds and independent reinspection make
accidental format drift visible, while stable JSON reports can be retained by
authors or consumed by their own local automation.

The EDK cannot establish authorship, licensing rights, intent, native-code
safety, live device/simulator support, or runtime correctness beyond its
bounded handshake. It does not sandbox hostile code or enforce CPU and memory
quotas. Public recommendation, signing, authenticated updates, revocation,
platform certification, and stronger isolation remain separate release gates.

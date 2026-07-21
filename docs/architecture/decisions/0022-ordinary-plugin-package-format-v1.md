# ADR-0022: Ordinary plugin package format version 1

- Status: Accepted
- Date: 2026-07-21

## Context

ADR-0021 requires independently installable extension artifacts without
requiring one payload language or one package kind. The first implemented
vertical slice needs a deterministic envelope for ordinary out-of-process
plugins, an offline installation path, and an explicit compatibility decision.

An archive is hostile input. Merely checking for `plugin.json` would permit
path traversal, symlinks, case collisions, undeclared payloads, decompression
abuse, ambiguous identities, and version replacement with different bytes.
Package integrity is also distinct from publisher identity: a digest can prove
that extracted bytes match an inventory without proving who created them.

## Decision

Ordinary plugin package schema version 1 uses a ZIP envelope with the
`.wyrmplugin` extension and media type
`application/vnd.wyrmgrid.plugin-package+zip`.

The archive contains `wyrmgrid-package.json` at its root. That manifest uses
`schemas/extension-package-manifest-v1.schema.json` and declares:

- package schema version `1`;
- package kind `ordinary_plugin`;
- the same reverse-domain identifier and three-part semantic version as the
  enclosed plugin manifest;
- the fixed plugin manifest path `plugin.json`; and
- an exact inventory of every payload file, including its byte length and
  lowercase SHA-256 digest.

The package manifest itself is not a payload inventory entry. No undeclared,
duplicate, case-colliding, directory, symlink, encrypted, absolute, traversing,
or platform-reserved archive entry is accepted. Version 1 accepts only stored
or Deflate compression and canonical ASCII package paths. Its limits are:

| Boundary                |                      Limit |
| ----------------------- | -------------------------: |
| Compressed archive      |                     32 MiB |
| Expanded payload        |                     64 MiB |
| Individual payload file |                     16 MiB |
| Package manifest        |                    256 KiB |
| Payload files           |                        512 |
| Path                    | 240 bytes and 8 components |
| Path component          |                   80 bytes |

WyrmGrid validates the entire archive and the existing `plugin.json` contract
before extraction. It then extracts into a fresh staging directory using
create-new file semantics, moves the validated tree into versioned managed
storage, and records its package manifest, plugin manifest, source, schema
version, and archive digest in append-only database migration 21.

An `(ordinary_plugin, id, version)` tuple is immutable. Reinstalling the same
version is accepted only when its archive digest is identical; different bytes
under the same identity and version fail as a conflict. Installing a new
version preserves the previous active version as a single-step rollback target
and preserves the user's enabled or disabled state. Disable changes discovery,
not package bytes. Removal first moves the extension tree to a tombstone,
revokes saved access, removes its database records, and then deletes the
tombstone; startup recovery resolves an interrupted removal against database
state.

Forge must inspect and display identity, author, version, runtime, file count,
sizes, and archive digest before a separate install confirmation. Installation
does not start a plugin and does not grant capabilities. Package schema version
1 carries no publisher signature, so `publisher_verified` is always false and
Forge warns the user to trust the file's source.

## Compatibility

Package schema versioning is independent from plugin API and runtime protocol
versioning. Introducing this envelope does not change plugin protocol version

1. A later additive package field may remain schema version 1 only if old
   validators can safely ignore it; the current manifest denies unknown fields,
   so practical extensions normally require a new package schema and an explicit
   compatibility decision.

Unknown package schema versions, package kinds, compression methods, runtimes,
or manifest locations remain inert. The ordinary-plugin contract in this
record deliberately accepts only `X.Y.Z` versions without prerelease or build
metadata. Simulator providers now have the separate version-one package-kind
contract in ADR-0022; audio providers still require their own decision rather
than widening this contract implicitly.

Local installation requires no Aerie account or network. A future signed Aerie
distribution record may bind this exact archive digest to a publisher, but it
must not reinterpret an unsigned local file as publisher-verified.

Installer seeding may advance an active first-party package only to a newer
first-party version. It never downgrades an installation and never replaces an
active version whose provenance is an explicit local-file install; that newer
seed is retained without silently taking control.

## Consequences

Community authors can distribute one inspectable file and users can install,
update, disable, roll back, or remove it without rebuilding WyrmGrid. Exact
inventory validation and immutable versions make local state reproducible and
prevent silent replacement.

ZIP parsing and file/database coordination become security-critical code.
Fixtures, adversarial archive tests, migration tests, Forge review, and threat-
model coverage are required for changes. Version 1 does not provide signatures,
revocation, resource quotas, or an operating-system sandbox; those limitations
remain visible rather than being inferred from the package envelope.

This decision implements the ordinary-plugin portion of
[ADR-0021](0021-externally-installable-extensions.md) and preserves the
out-of-process boundary in [ADR-0002](0002-out-of-process-plugins.md).

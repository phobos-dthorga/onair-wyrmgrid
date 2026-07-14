# ADR-0013: SQLCipher, device-bound keys, and portable backups

## Status

Accepted.

## Context

Hoard history, simulator recordings, authorization decisions, legal choices,
and imported customisation data share WyrmGrid's local SQLite database. Normal
filesystem permissions are useful but do not protect a copied database from an
offline reader. Binding encryption only to one installation, however, would
make legitimate backup, operating-system reinstall, and device-migration flows
impractical.

WyrmGrid has no public user population or released plaintext database that must
be migrated. Shipping a one-off plaintext import path before release would add
a permanent downgrade and attack surface without preserving real user data.

## Decision

Persistent WyrmGrid storage uses SQLCipher Community Edition through
`rusqlite`'s bundled SQLCipher and vendored OpenSSL build. A new installation
generates a cryptographically secure 256-bit database key. The key is stored by
the operating-system credential service and never beside the database, in
settings, diagnostics, plugin messages, or source control.

Startup fails closed when encrypted database state exists but the matching key
is missing, malformed, or inaccessible. WyrmGrid does not generate a replacement
key, retry as plaintext SQLite, or silently switch to memory-only storage. A new
database is created only when neither active nor recovery database state exists.

Portability uses a separate, versioned encrypted backup:

- the user chooses a dedicated password of at least 12 characters;
- SQLCipher exports a consistent snapshot to a new file without exposing the
  device key;
- WyrmGrid refuses to overwrite an existing destination;
- the encrypted manifest records backup-format version, database schema,
  creation time, and application version;
- restore verifies the password, encrypted manifest, supported versions, and
  database integrity before staging anything; and
- the validated backup is re-encrypted with the destination device's key,
  activated at the next launch, and guarded by a rollback copy until the new
  database opens successfully.

Backup-format version 1 restores the complete WyrmGrid database. Selective
export is intentionally not implied. The password is held only for the active
operation and is not stored or recoverable by WyrmGrid. The interface exposes
only the selected filename after the operating-system picker returns, and the
`data-protection-` localisation namespace is protected from unreviewed packs.

Plaintext SQLite compatibility is deliberately broken before first public
release. There is no automatic migration, plaintext detector, or fallback. A
future released format change must use a new documented, tested compatibility
decision rather than weakening this boundary.

## Consequences

- A copied `wyrmgrid.db` is not useful without the separate device key, but an
  attacker controlling a logged-in process or operating-system account may be
  able to ask the credential service for that key.
- Losing the operating-system credential entry makes the local database
  unrecoverable. A portable backup and its password are the supported recovery
  path.
- A weak or disclosed backup password weakens the portable copy. WyrmGrid
  cannot reset it, and user-controlled cloud, removable-media, and deletion
  behaviour remains outside the application.
- Encryption at rest does not remove plaintext from process memory, UI fields,
  virtual memory, crash dumps, screenshots, or data deliberately exported by a
  user or authorised integration.
- The vendored cryptography build increases clean-build time and requires Perl
  plus a native C toolchain; release artefacts must carry SQLCipher and OpenSSL
  notices.
- Backup and restore run away from the UI thread. Restore remains an explicit
  destructive confirmation rather than a durable capability grant.

## Follow-up decisions

- Establish signed release backup-compatibility fixtures across supported
  WyrmGrid versions.
- Decide whether a later format supports selective export without creating
  misleading completeness or provenance claims.
- Review platform credential-store behaviour, unattended/headless sessions,
  and recovery messaging on every supported operating system before stable
  release.
- Add a deliberate cancel-pending-restore control if real use shows that
  restart-only activation is insufficient.

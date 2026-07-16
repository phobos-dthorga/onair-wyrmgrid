# ADR-0014: Optional OS-protected provider accounts

**Status:** Accepted

## Context

ADR-0006 deliberately began with session-only OnAir credentials while the
desktop boundary was young. WyrmGrid now has SQLCipher-encrypted preferences,
a reviewed operating-system credential adapter, explicit backup semantics, and
user-facing controls for forgetting saved values.

Users reasonably want to avoid re-entering an OnAir API key and a SimBrief
Pilot ID on every launch. Those two values are not equivalent: the OnAir API
key authenticates API requests, while SimBrief's latest-OFP endpoint uses a
Pilot ID or username as an account reference and does not require the user's
SimBrief or Navigraph password.

## Decision

Remembering either provider is optional and visibly controlled by the user.

- The OnAir API key is stored only by the operating-system credential service
  under WyrmGrid's application identity. It is never stored in SQLite, browser
  storage, diagnostics, plugins, portable backups, source, or CI.
- The matching OnAir Company ID and the separate, default-off automatic-connect
  choice are metadata in WyrmGrid's SQLCipher database.
- A remembered SimBrief Pilot ID or username is non-secret but private account
  metadata in the SQLCipher database. WyrmGrid never asks for or stores a
  SimBrief or Navigraph password.
- Interface commands return only profile availability and non-secret metadata.
  A remembered OnAir key moves directly from the OS store into the Rust
  connection service and never returns to the webview.
- Disconnecting ends the active OnAir session without silently deleting the
  saved profile. **Forget saved details** is a separate operation.
- Automatic OnAir connection runs only after the current legal documents have
  been acknowledged and only when the user separately enables it.
- Portable backups contain provider metadata, including Company ID, SimBrief
  account reference, and startup choice, but never contain the OnAir API key.
  A restored OnAir profile therefore requires the key to be entered again on
  the destination device.

Saving follows a validate-first rule: WyrmGrid first proves that the entered
OnAir details can connect, then writes the OS secret and encrypted metadata. If
metadata persistence fails, it attempts to remove the newly written secret.
Missing or unavailable OS credentials fail closed and never fall back to
plaintext storage.

## Consequences

Session-only use remains available. Remembered credentials improve usability
without making CI, installers, or the encrypted database a key-distribution
mechanism. Cross-device recovery intentionally requires both a portable backup
and re-entry of the OnAir API key.

The desktop must maintain platform-specific credential-store adapters and test
missing-entry, unavailable-store, save, replace, forget, and partial-failure
paths before broad distribution. Future OAuth tokens follow this same secret
boundary only after the provider approves a native desktop flow.

This decision supersedes ADR-0006 for builds that implement optional
credential persistence; ADR-0006 remains the historical record for the initial
session-only milestone.

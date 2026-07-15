# Remembered accounts and credentials

WyrmGrid can remember provider details only when you ask it to. Session-only
use remains available, and automatic connections are off by default.

## OnAir

Open **OnAir connection** and enter the Company ID and API Key from **OnAir
Client → Options → Global Settings**.

- Leave **Remember this connection** clear to keep the connection in memory
  only. Closing WyrmGrid forgets the key.
- Select it to let Windows Credential Manager remember the API key. WyrmGrid's
  encrypted database stores the Company ID and your startup choice, not the
  API key.
- **Connect automatically when WyrmGrid starts** is a separate choice and is
  off by default. If enabled, WyrmGrid contacts OnAir only after the current
  Privacy Notice and Terms have been accepted.
- **Disconnect** ends the live session but keeps remembered details.
- **Forget saved details** removes the Windows credential and WyrmGrid's saved
  Company ID. An already connected session remains active until you disconnect
  or close WyrmGrid.

The API key is never sent to plugins, diagnostics, Sentry, or a portable
backup. If Windows Credential Manager is unavailable or has lost the entry,
WyrmGrid does not fall back to plaintext storage: enter the key again or use a
session-only connection.

## SimBrief

Dispatch can remember the Pilot ID—or username when that reference mode is
chosen—after a successful **Import latest OFP**.

This reference is not a password or API key. It is saved in the encrypted local
database only when **Remember this account reference** is selected. Clearing
that choice during a successful import removes the saved reference. The
current imported plan remains session-only unless it becomes associated with a
telemetry recording under the documented recording rules.

WyrmGrid never asks for a SimBrief or Navigraph password. The latest-OFP flow
sends the selected Pilot ID or username to SimBrief only when an import is
requested.

## Backups, migration, and reinstalls

A portable `.wyrmbackup` includes the OnAir Company ID, automatic-connect
choice, and remembered SimBrief reference because they are encrypted database
metadata. It does not include the OnAir API key held by Windows.

After restoring on another computer or reinstalling Windows, review the saved
metadata and enter the OnAir API key again. This avoids turning a portable
database backup into a transferable authentication secret. Whole-system backup
software may restore Windows credentials separately, but WyrmGrid does not
rely on that provider-specific behaviour.

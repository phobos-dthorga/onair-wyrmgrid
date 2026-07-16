# Core authorization model

Authorization is an internal Rust application service. Feature services ask it
for a decision; Tauri and Svelte only carry requests and present results.

## Decision taxonomy

| Decision                      | Example                                         | Persistence                | Authority created                     |
| ----------------------------- | ----------------------------------------------- | -------------------------- | ------------------------------------- |
| Legal acknowledgement         | Current Terms and Privacy Notice                | Versioned local record     | None beyond recording acknowledgement |
| Feature consent or preference | Optional diagnostics                            | Local preference           | Only the named core behaviour         |
| Capability grant              | Plugin reads sanitized live simulator telemetry | Once, session, or standing | Only named capabilities               |
| Momentary confirmation        | Delete all completed recordings                 | Not reusable               | One attempted action                  |

The categories never imply one another. In particular:

- accepting legal documents does not approve a plugin;
- enabling diagnostics does not permit arbitrary data access;
- starting or retaining a recording does not grant historical telemetry;
- provider capability negotiation does not represent user consent; and
- confirming deletion does not create a standing delete permission.

## Enforcement rules

1. Start from denial when a grant cannot be read, validated, or matched.
2. Bind every grant to a validated actor identifier, exact capability,
   and scope revision.
3. Re-check grants at the operation boundary, not only when drawing the UI.
4. Treat actor version or requested-capability changes as a new review scope.
5. Revoke active access before removing its persisted grants.
6. Store only symbolic identifiers and bounded decision metadata in the local
   audit trail; retain only the newest 4,096 decisions and never store API keys,
   raw payloads, or plugin output there.
7. Keep protected permission, credential, legal, telemetry, Security Centre,
   destructive-action, and error wording under canonical WyrmGrid control.
8. Consume an `allow once` grant at the privileged operation boundary, retain a
   session grant only in the shared in-memory authorization runtime, and persist
   only a standing grant.

The initial subject kind is `plugin`. Adding an in-game client, provider write
capability, notification sender, or external-network proxy requires an explicit
subject kind and operation-specific capabilities. Generic `read_all`,
`filesystem`, `network`, or `simulator_control` grants are intentionally
rejected as architectural shortcuts.

## Current plugin compatibility

Migration 9 introduces `authorization_grants` and append-only
`authorization_decisions`. The application stops consulting the older plugin
grant table. Existing preview grants are therefore denied until the user reviews
the current plugin version and exact requested permission set again.

The present plugin manifest treats every requested permission as required for
launch. The core can store a subset, but Forge approves the complete reviewed
set because a partially approved plugin could not currently start. Optional
capabilities require a future protocol decision and fixtures before the UI
offers per-capability toggles.

Forge now offers three explicit lifetimes for that complete reviewed set:

- **One launch** authorizes one supervised plugin launch attempt and is consumed
  before the child receives WyrmGrid data;
- **This WyrmGrid session** permits repeated launches until the application
  closes and is never written to SQLite; and
- **Until revoked** creates the revision-bound encrypted standing grant.

The running-plugin record retains the lifetime and exact capabilities used for
that process, so consuming a one-launch grant cannot make a live child appear
unreviewed. A new application process starts with no temporary grants.

## User-facing Security Centre

The first Security Centre slice is available from **Settings > Security &
permissions**. It reads a bounded application-owned view from the same Rust
authorization service and shows:

- current Terms, Privacy Notice, and optional diagnostics status;
- actors with active grants, their exact scope revision, capability names,
  lifetime, and grant time;
- the newest 100 symbolic grant/revoke decisions while storage retains at most
  4,096; and
- a plugin revocation action routed through Forge so an active child process is
  stopped before its persisted authority is removed.

The interface does not query SQLite directly and is not an enforcement
boundary. It receives no credentials, raw OnAir or provider payloads, plugin
messages, or historical simulator samples. Unknown or malformed actor kinds,
scopes, capabilities, counts, or audit decisions make the view fail visibly
rather than being presented as trusted history.

## Suggested next security slices

1. Define signed publisher identity before deciding whether an unchanged grant
   may survive safe plugin updates.
2. Add integration tests proving every Tauri command for privileged work fails
   when called directly without authorization, regardless of UI state.
3. Add wall-clock expiry only for an actor and capability with a demonstrated
   need; session lifetime must not be presented as elapsed-time expiry.
4. Add filters, decision-detail explanations, and actor publisher identity only
   after those fields have stable, privacy-reviewed contracts.

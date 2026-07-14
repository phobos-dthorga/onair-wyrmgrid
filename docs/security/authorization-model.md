# Core authorization model

Authorization is an internal Rust application service. Feature services ask it
for a decision; Tauri and Svelte only carry requests and present results.

## Decision taxonomy

| Decision                      | Example                                         | Persistence                       | Authority created                     |
| ----------------------------- | ----------------------------------------------- | --------------------------------- | ------------------------------------- |
| Legal acknowledgement         | Current Terms and Privacy Notice                | Versioned local record            | None beyond recording acknowledgement |
| Feature consent or preference | Optional diagnostics                            | Local preference                  | Only the named core behaviour         |
| Capability grant              | Plugin reads sanitized live simulator telemetry | Subject- and revision-bound grant | Only named capabilities               |
| Momentary confirmation        | Delete all completed recordings                 | Not reusable                      | One attempted action                  |

The categories never imply one another. In particular:

- accepting legal documents does not approve a plugin;
- enabling diagnostics does not permit arbitrary data access;
- starting or retaining a recording does not grant historical telemetry;
- provider capability negotiation does not represent user consent; and
- confirming deletion does not create a standing delete permission.

## Enforcement rules

1. Start from denial when a grant cannot be read, validated, or matched.
2. Bind every durable grant to a validated actor identifier, exact capability,
   and scope revision.
3. Re-check grants at the operation boundary, not only when drawing the UI.
4. Treat actor version or requested-capability changes as a new review scope.
5. Revoke active access before removing its persisted grants.
6. Store only symbolic identifiers and bounded decision metadata in the local
   audit trail; retain only the newest 4,096 decisions and never store API keys,
   raw payloads, or plugin output there.
7. Keep protected permission, credential, legal, telemetry, Security Centre,
   destructive-action, and error wording under canonical WyrmGrid control.

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

## User-facing Security Centre

The first Security Centre slice is available from **Settings > Security &
permissions**. It reads a bounded application-owned view from the same Rust
authorization service and shows:

- current Terms, Privacy Notice, and optional diagnostics status;
- actors with active grants, their exact scope revision, capability names, and
  grant time;
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

1. Add session-only and time-limited grants before introducing actors that can
   write, send notifications, or reach external networks.
2. Define signed publisher identity before deciding whether an unchanged grant
   may survive safe plugin updates.
3. Add explicit `allow once`, `allow until WyrmGrid closes`, and `always allow`
   semantics only when an actual capability benefits from each lifetime.
4. Add integration tests proving every Tauri command for privileged work fails
   when called directly without authorization, regardless of UI state.
5. Add filters, decision-detail explanations, and actor publisher identity only
   after those fields have stable, privacy-reviewed contracts.

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
7. Keep protected permission, credential, legal, telemetry, destructive-action,
   and error wording under canonical WyrmGrid control.

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

## Suggested next security slices

1. Add a Security Centre that lists actors, grant scope, grant time, and a
   revoke action without exposing raw data.
2. Add session-only and time-limited grants before introducing actors that can
   write, send notifications, or reach external networks.
3. Define signed publisher identity before deciding whether an unchanged grant
   may survive safe plugin updates.
4. Add explicit `allow once`, `allow until WyrmGrid closes`, and `always allow`
   semantics only when an actual capability benefits from each lifetime.
5. Add integration tests proving every Tauri command for privileged work fails
   when called directly without authorization, regardless of UI state.

# ADR-0012: Core authorization and distinct consent boundaries

## Status

Accepted.

## Context

WyrmGrid now has versioned legal acknowledgement, optional diagnostics,
out-of-process plugin permissions, simulator-provider capability negotiation,
manual telemetry recording, and destructive confirmations. Treating all of
these as generic checkboxes would make it too easy for one decision to be
mistaken for authority in another security domain.

Permission checks were also beginning inside feature services. Plugin grants,
for example, were read and written directly by the plugin supervisor. That
would not scale safely to future in-game clients, additional simulator
consumers, or privileged core integrations.

## Decision

WyrmGrid owns authorization and consent policy in the Rust application core.
Tauri commands and Svelte components remain adapters and never decide whether
an operation is authorized.

The policy vocabulary distinguishes four decisions:

1. **Legal acknowledgement** records acceptance of specific document versions.
2. **Feature consent or preference** controls optional core behaviour such as
   diagnostics or future automatic recording.
3. **Capability grants** authorize a named non-user actor to perform a bounded
   operation against a precise scope revision.
4. **Momentary confirmation** authorizes one destructive or sensitive action
   and is not retained as a reusable grant.

Durable capability grants are deny-by-default and keyed by subject kind,
subject identifier, capability, and scope revision. Grant and revoke decisions
produce local audit entries bounded to the newest 4,096 decisions. A changed plugin version or requested
permission set has a new scope revision and therefore requires review again.
Revocation stops an active plugin before its grants are removed.

Migration `0009_authorization.sql` creates the new grant and decision tables.
The earlier `plugin_permission_grants` table remains untouched because shipped
migrations are append-only, but the core no longer treats those preview-era
rows as current authorization. This is an explicit fail-closed compatibility
decision: users must approve installed plugins once more.

Provider capability negotiation proves protocol compatibility; it is not a
user grant. Manual **Start recording** is a momentary user action; it does not
grant plugins access to live or historical telemetry. Diagnostics consent does
not authorize feature data sharing.

## Consequences

- New privileged actors must integrate with the core authorization service
  rather than add feature-local permission storage.
- UI wording can explain or request a decision, but hiding, enabling, or
  disabling a button is never the enforcement boundary.
- Permission-set and actor-version changes fail closed.
- Security review can distinguish what was acknowledged, preferred, granted,
  or confirmed.
- Existing preview plugin grants require fresh approval after migration 9.

## Follow-up decisions

- Whether future capabilities may be optional in plugin manifests rather than
  all requested capabilities being required for launch.
- Whether grants may be session-only, time-limited, or always persistent.
- Whether signed publisher identity can safely allow grants to survive a plugin
  version update with an unchanged permission set.
- What a future user-visible Security Centre should expose without leaking
  sensitive operational history into support captures.

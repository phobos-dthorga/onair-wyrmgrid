# Plugin platform

WyrmGrid plugins are separate processes. The initial transport will use framed
JSON messages over standard input and output; JSON-RPC semantics may be adopted
where they improve tooling, but transport framing and lifecycle messages must be
specified before plugins execute.

`plugin.json` declares identity, compatibility, entry point, and requested
permissions. The host validates it before launch. A manifest is not a sandbox:
process isolation, operating-system controls, message validation, timeouts, and
user trust decisions remain necessary.

The version-one manifest groundwork is in
`schemas/plugin-manifest.schema.json`, mirrored by Rust types in
`wyrmgrid-plugin-protocol`. The example is intentionally non-executable until
the lifecycle and framing contract are accepted.

## Chart contributions

Charts use the versioned, declarative contract in
`schemas/chart-spec.schema.json`. A plugin granted `charts_publish` may provide
validated series data for line, area, or bar charts. The host controls rendering
and does not accept ECharts configuration, JavaScript functions, HTML tooltips,
or plugin-defined themes.

The fixture in `schemas/fixtures/chart-spec-v1.json` is the canonical version
one example. The Rust protocol crate deserializes and validates it in tests.
Chart schema version 1 was added compatibly to plugin API version 1: existing
plugins do not need to request the new permission or emit chart messages.

## External integration capabilities

Provider adapters belong to the host. Plugins consume only stable, sanitized
operational snapshots after the corresponding public capability is specified.
They do not receive raw SimBrief, SayIntentions.AI, weather, VATSIM, IVAO,
Navigraph, OnAir, or simulator payloads and cannot borrow the host's provider
credentials.

SayIntentions reads and actions are not covered by `external_network`,
`notifications_create`, or `simulator_telemetry_read`. A future capability must
name the bounded operation, keep the account key in the host, apply host-owned
message templates and limits, and require user confirmation for every external
effect unless a separately reviewed automation rule exists.

The manifest's existing `simulator_telemetry_read` permission is reserved for a
future bounded WyrmGrid Bridge snapshot. It does not grant simulator commands,
flight-plan loading, raw SimConnect or FSUIPC access, arbitrary dataref access,
or historical tracks. Those would require separate capabilities and protocol
reviews.

The version-one `external_network` name must not be interpreted as unrestricted
internet access or a host endpoint proxy. No runtime implements it yet. Before a
plugin runtime ships, the project must either define a destination- and
operation-scoped broker or supersede the value with narrower provider
capabilities through an explicit plugin-protocol compatibility decision.

Likewise, `notifications_create` permits a bounded host notification request; it
does not authorize Discord, email, webhook, calendar, or arbitrary network
delivery. Community-delivery plugins need destination-specific user approval
and keep their own service credentials outside host snapshots.

See the [external integrations programme](../integrations/README.md) for planned
provider boundaries.

## Planned first-party demonstrations

The [Operational Planner concept](operational-planner.md) is a planned flagship
plugin with Charter Desk and Airline Network workspaces. It is intentionally
later than the small idle-aircraft plugin proof: the planner should exercise a
proven public plugin surface, not cause private shortcuts to be added for one
ambitious first-party feature.

A mature first-party plugin may be promoted into WyrmGrid's primary navigation
without being moved into the core or granted private access. Visible prominence
is a product decision; protocol, permission, storage, and lifecycle boundaries
remain those of a plugin.

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

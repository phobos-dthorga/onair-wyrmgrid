# Plugin platform

WyrmGrid plugins are separate processes. Protocol version 1 uses length-prefixed
JSON messages over standard input and output, a versioned startup handshake,
monotonic message sequences, and a supervised shutdown. The host owns
rendering, credentials, stable provider contracts, permission persistence, and
process state. A weather provider plugin owns only request URL construction and
translation of its raw response into those stable contracts.

`plugin.json` declares identity, compatibility, entry point, and requested
permissions. The host validates it before launch. A manifest is not a sandbox:
process isolation, operating-system controls, message validation, timeouts, and
user trust decisions remain necessary.

The version-one manifest is in
`schemas/plugin-manifest.schema.json`, mirrored by Rust types in
`wyrmgrid-plugin-protocol`. An optional `runtime` field was added compatibly to
plugin API version 1; manifests without it remain valid metadata but cannot be
started. The executable Python proof is
`examples/plugins/fleet-locations`, with its zero-dependency SDK in
`sdk/python`.

Forge presents each plugin's requested and granted capabilities. Grants are
empty by default and stored locally; the current proof requires every requested
capability to be approved before launch. `on_air_fleet_read`,
`simulator_telemetry_read`, `map_layers_publish`, `external_network`, and
`weather_data_publish` execute in this slice. The two weather permissions are
accepted only together with a declared product capability and exact HTTPS
origin. A
plugin receives only the stable translated snapshots it was granted and
publishes data-only point layers that Atlas renders using the active host theme.

Grant enforcement is owned by the core authorization service. Approval is
bound to the plugin identifier, version, and exact requested permission set.
Changing the version or requested permissions requires a fresh review. The
append-only migration-4 preview grant table is no longer authoritative after
migration 9, so an existing preview installation asks once more rather than
silently carrying an unverifiable grant forward.

Forge can also remember **Start automatically with WyrmGrid** for an individual
plugin. This host-owned preference is off by default and can be enabled only
after standing access has been approved. It is bound to the same plugin version,
capability, weather-product, and network-origin scope as the approval; changing
any of those details makes the saved startup choice inactive until the user
reviews the plugin and enables it again. Temporary once or session access can
never authorize a future launch. A manual stop affects the current session only
and does not silently rewrite the saved choice.

Automatic launches occur independently after a normal startup and current legal
acknowledgement. One missing runtime, invalid plugin, or failed handshake is
reported against that plugin and does not prevent other enabled plugins or the
core application from starting. The preference belongs to WyrmGrid's encrypted
database rather than `plugin.json`, so a plugin cannot enable itself. Revoking
access removes the corresponding automatic-start choice.

The complete framing, lifecycle, limit, and compatibility contract is in
[protocol version 1](protocol-v1.md).

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

Stable provider contracts, validation, scheduling, caching, and presentation
belong to the host. Weather provider plugins may fetch only for a correlated
host request and publish a normalized airport snapshot, numeric grid, or PNG
tile layer. They do not receive raw SimBrief, SayIntentions.AI, VATSIM, IVAO,
Navigraph, OnAir, or simulator payloads and cannot borrow host credentials.

SayIntentions reads and actions are not covered by `external_network`,
`notifications_create`, or `simulator_telemetry_read`. A future capability must
name the bounded operation, keep the account key in the host, apply host-owned
message templates and limits, and require user confirmation for every external
effect unless a separately reviewed automation rule exists.

The manifest's `simulator_telemetry_read` permission now delivers the current
bounded, versioned WyrmGrid Bridge snapshot when one exists. It does not grant
simulator commands, flight-plan loading, provider selection, raw SimConnect or
FSUIPC access, arbitrary dataref access, or historical tracks. Those require
separate capabilities and protocol reviews.

The version-one `external_network` name must not be interpreted as unrestricted
internet access or a host endpoint proxy. WyrmGrid grants it in this slice only
to a weather plugin that declares bounded product capabilities and exact HTTPS
origins. The bundled SDK enforces those origins, but the current Python
developer preview is not an operating-system sandbox: a process may retain
ambient access available to the user's account.
Only trusted plugin code should be run. Before community distribution, the
project must add reviewed OS isolation or define a destination- and
operation-scoped broker, and should supersede this broad name with narrower
provider capabilities through an explicit compatibility decision.

Likewise, `notifications_create` permits a bounded host notification request; it
does not authorize Discord, email, webhook, calendar, or arbitrary network
delivery. Community-delivery plugins need destination-specific user approval
and keep their own service credentials outside host snapshots.

See the [external integrations programme](../integrations/README.md) for planned
provider boundaries and
[simulator provider authoring](../integrations/simulator-provider-authoring.md)
for the distinction between providers and ordinary plugins.

## First-party weather providers

The independently installable providers in `plugins/` are:

- Open-Meteo for a coarse, host-selected global model grid;
- AviationWeather.gov for explicit plan-airport METAR and TAF requests; and
- RainViewer for a small host-selected current global radar tile set.

They share the same SDK and stable core weather models. Provider failures are
independent, the last valid global layer remains visible when a refresh fails,
and stopping a plugin removes its active contribution.

## Planned first-party demonstrations

The [Operational Planner concept](operational-planner.md) is a planned flagship
plugin with Charter Desk and Airline Network workspaces. It is intentionally
later than the small Fleet Locations plugin proof: the planner should exercise a
proven public plugin surface, not cause private shortcuts to be added for one
ambitious first-party feature.

A mature first-party plugin may be promoted into WyrmGrid's primary navigation
without being moved into the core or granted private access. Visible prominence
is a product decision; protocol, permission, storage, and lifecycle boundaries
remain those of a plugin.

# Plugin platform

WyrmGrid plugins are separate processes. Protocol version 1 uses length-prefixed
JSON messages over standard input and output, a versioned startup handshake,
monotonic message sequences, and a supervised shutdown. The host owns
rendering, credentials, stable provider contracts, permission persistence, and
process state. A weather provider plugin owns only request URL construction and
translation of its raw response into those stable contracts.

## External delivery invariant

Every WyrmGrid component described as a plugin or provider must be deliverable
as an external artifact. Installing, replacing, disabling, updating, or
removing it must not require compiling or relinking the WyrmGrid desktop.
First-party and community packages use the same public contracts and lifecycle;
an official installer may seed first-party packages as a convenience, but no
plugin may exist only as compile-time embedded code.

The payload format follows the integration need rather than one mandated
implementation language. A package may contain Python or another supported
script runtime, a standalone executable, a platform-native library loaded by a
simulator, or a future validated runtime such as WebAssembly. Native code must
never be loaded into the WyrmGrid desktop process: when a simulator requires an
in-process module, that module belongs to the simulator-facing package and
communicates with a separately supervised WyrmGrid provider.

Format flexibility does not bypass trust controls. Each supported package kind
needs a versioned manifest, bounded contents, a declared entry point and
runtime, compatibility metadata, complete capabilities and network origins,
deterministic validation, and safe install, update, rollback, disable, and
removal behaviour. Unknown formats remain inert until WyrmGrid has an explicit
adapter and compatibility decision.

Local installation is a core offline capability and must not depend on WyrmGrid
Aerie, an account, or a network connection. Aerie may later add discovery,
publisher identity, signatures, revocation metadata, and convenient updates;
it is not the source of the plugin model or a prerequisite for loading an
already verified local package. This direction is recorded in
[ADR-0020](../architecture/decisions/0020-externally-installable-extensions.md).

The current developer preview implements the first ordinary-plugin vertical
slice. Forge can inspect and install a local `.wyrmplugin` file without Aerie,
store immutable versions outside the executable, activate a new version while
retaining one rollback target, disable or enable discovery, and remove the
managed package. Installation does not start the process or approve its
capabilities. The four first-party Python plugins are now deterministic,
separately distributable `.wyrmplugin` files seeded through the same managed
installer. Simulator sidecars now use a distinct `.wyrmprovider` contract and
managed lifecycle; audio providers remain the next migration boundary.

## Ordinary plugin packages

Package schema version 1 is a bounded ZIP envelope with the `.wyrmplugin`
extension. Its root `wyrmgrid-package.json` identifies an
`ordinary_plugin`, fixes the enclosed manifest at `plugin.json`, and inventories
every payload file with its size and lowercase SHA-256 digest. WyrmGrid rejects
undeclared files, digest or identity mismatches, traversal, links, directories,
case collisions, encrypted entries, unsupported compression, excessive sizes,
and unknown schema versions before extraction.

To install a local package:

1. open **WyrmGrid Forge** and choose **Choose package…**;
2. select a `.wyrmplugin` file;
3. review its identity, author, version, runtime, size, file count, and complete
   archive digest;
4. heed the unsigned-package warning and confirm installation only when the
   file's source is trusted; and
5. review capabilities separately before starting the installed plugin.

The package format proves content consistency, not publisher identity. Schema
version 1 is unsigned and Forge always reports its publisher as unverified.
The exact compatibility decision, limits, managed lifecycle, and security
consequences are in
[ADR-0021](../architecture/decisions/0021-ordinary-plugin-package-format-v1.md).
The canonical package manifest schema and fixture are
`schemas/extension-package-manifest-v1.schema.json` and
`schemas/fixtures/extension-package-manifest-plugin-v1.json`.

Authors can build the envelope from an existing plugin directory without
compiling WyrmGrid:

```powershell
npm run plugin:package -- --source path\to\plugin --output dist\my-plugin.wyrmplugin
```

The packager refuses to overwrite an output unless `--force` is explicit. A
shared runtime file can be placed at a canonical package path with a repeatable
`--include SOURCE=PACKAGE_PATH` argument. For example, the current zero-
dependency Python SDK may be added as
`--include sdk/python/wyrmgrid_sdk/__init__.py=src/wyrmgrid_sdk/__init__.py`.
The command validates paths, identity, version, entry-point presence, file and
archive limits, creates a complete digest inventory, and emits deterministic
ZIP bytes. The desktop independently performs the full validation again; the
authoring command is not a trust decision.

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

Forge also exposes host-owned configuration for bounded, non-secret behaviour.
The first settings control how often WyrmGrid asks forecast-grid and RADAR
providers for refreshed layers. Definitions, allowed choices, validation,
scheduling, and rendering remain in the host; plugins cannot declare arbitrary
controls, read these records, write them, or use this mechanism for credentials.
The values live in the encrypted WyrmGrid database and do not alter plugin API
version 1 or its messages. This provides a safe base for future host-controlled
plugin options without making `plugin.json` an application-settings surface.

The complete framing, lifecycle, limit, and compatibility contract is in
[protocol version 1](protocol-v1.md).

WyrmGrid Aerie is a future distribution and discovery service, not part of the
current protocol proof and not a prerequisite for local package installation.
Its proposal keeps upload quarantine, validation, moderation, publisher
signatures, repository approval, desktop verification, installation, and
rollback as distinct steps. See
[ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
and the
[hosted-platform implementation plan](../operations/hosted-platform.md).

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
for the distinct `.wyrmprovider` envelope, native trust warning, and Bridge
lifecycle.

## First-party weather providers

The first-party provider source packages in `plugins/` are:

- Open-Meteo for a coarse, host-selected global model grid;
- AviationWeather.gov for explicit plan-airport METAR and TAF requests; and
- RainViewer for a small host-selected recent global RADAR timeline with
  explicit no-coverage masks.

They share the same SDK and stable core weather models. Provider failures are
independent, the last valid global layer remains visible when a refresh fails,
and stopping a plugin removes its active contribution.

For a completed imported plan, the host may add a bounded UTC window to the
existing airport-report and forecast-grid request. The AviationWeather.gov
provider returns actual METAR observations inside that window and omits TAF;
the Open-Meteo provider uses its separately approved Historical Forecast
origin and labels the result `historical_model`. A current request still omits
the window, preserving the version-one live request shape. The host rejects a
historical response whose time scope does not match the correlated request.

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

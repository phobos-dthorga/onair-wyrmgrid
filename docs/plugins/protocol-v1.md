# Plugin protocol version 1

## Compatibility decision

The executable runtime is an additive extension of plugin API version 1. The
manifest's optional `runtime` field and the framed lifecycle messages were added
before any public executable plugin contract existed. Existing version-one
metadata remains valid, but a plugin without `runtime: "python"` is not
executable. Removing fields, changing their meaning, or changing framing would
require a new protocol or plugin API version.

The provenance vocabulary later gained the additive
`external_calculation` value so provider-produced calculations remain distinct
from both external observations and WyrmGrid calculations. Version-one fleet
messages do not emit that value; plugin and chart schemas accept it for honest
host-validated contributions.

The `simulator_telemetry_snapshot` host message is another additive version-one
extension. Its manifest permission was already defined, and plugins that do not
request `simulator_telemetry_read` never receive the message. Existing plugins
therefore remain compatible. Raw provider payloads and simulator writes are not
part of plugin protocol version 1.

The weather-provider boundary is also additive within version one. Manifests
may declare `weather_data_publish`, one or more product capabilities, and exact
HTTPS origins. The host sends `weather_request` only to a running, approved
provider with the matching capability, and accepts `publish_weather` only for
an outstanding request. Plugins using the earlier message set do not request
these permissions, receive no weather requests, and remain compatible. The
host-selected station, grid-point, or tile set is part of the request contract;
a response that widens or substitutes it is rejected.

The additive RADAR-history request field is `frame_offset`, an integer from
`0` through `5`; `0` means the newest past frame. It is omitted for the legacy
current-frame request shape and is sent only to the bundled RainViewer plugin,
so existing version-one community plugins receive no new request member. A
raster tile may add `coverage_png_base64`; it is validated as a second bounded
256×256 PNG and counts toward the unchanged 640 KiB decoded layer ceiling.
Coverage is a factual no-data mask, not an alternate weather image. The
sanitized request and response fixtures are
`plugin-radar-history-request-v1.json` and `plugin-radar-layer-v1.json`.

The canonical manifest schema is `schemas/plugin-manifest.schema.json`. The
envelope schema and accepted examples are
`schemas/plugin-protocol-envelope.schema.json` and `schemas/fixtures/plugin-*-v1.json`.
Rust tests deserialize the fixtures and run the same validation used by the
host.

## Transport

Each message is:

1. a four-byte unsigned big-endian payload length;
2. that many bytes of UTF-8 JSON; and
3. no delimiter or trailing data.

The maximum JSON payload is 1 MiB. Zero-length, oversized, truncated, invalid
UTF-8, and invalid JSON frames are rejected. Standard output is protocol-only;
plugin diagnostics must not be mixed into it.

Every JSON payload is wrapped in:

```json
{
  "protocol_version": 1,
  "sequence": 1,
  "payload": { "type": "..." }
}
```

Host-to-plugin and plugin-to-host sequences are independent, begin at 1, and
must strictly increase. Unknown protocol versions or repeated/out-of-order
sequences stop the plugin.

## Lifecycle

1. WyrmGrid validates the manifest, canonical plugin directory, and entry point.
2. The user approves every requested capability in Forge.
3. WyrmGrid starts Python 3 in isolated mode with a scrubbed environment and
   piped standard input/output.
4. The host sends `hello` with the expected plugin ID, exact grants, declared
   weather capabilities, and approved HTTPS origins.
5. The plugin has three seconds to return a matching `ready` message with plugin
   API version 1.
6. For each granted read capability, the host sends the latest stable fleet or
   simulator snapshot when one exists. It sends later fleet observations when
   their timestamp changes and later simulator observations when their sequence
   advances. These local checks do not trigger OnAir or simulator requests.
7. Forge may send `shutdown`; WyrmGrid gives the child a short graceful window,
   then terminates it. Application exit also terminates supervised children.

An invalid handshake, malformed frame, unauthorized message, invalid layer, or
unexpected exit moves the process to a bounded failed state. Raw child output is
not relayed to the UI, logs, or telemetry.

## Host messages

- `hello`: host version, expected plugin ID, and granted capabilities.
- `fleet_snapshot`: company display identity, translated aircraft summaries,
  source provenance, observation time, and live/cached/offline availability.
- `simulator_telemetry_snapshot`: current translated simulator identity,
  aircraft, position, motion, fuel/weight, lifecycle flags, and source
  provenance. Optional facts remain absent when the provider cannot establish
  them.
- `weather_request`: a correlated, bounded request for up to ten airport
  reports, 512 host-selected model samples, or 16 host-selected raster tiles.
- `shutdown`: request for an orderly exit.

Snapshots never contain the OnAir API key, raw OnAir JSON, raw SimConnect
variables, FSUIPC offsets, provider paths, or native error text. Provider models
are translated into stable WyrmGrid domain summaries before this adapter.

## Plugin messages

- `ready`: exact plugin ID and supported plugin API version.
- `publish_map_layer`: one host-rendered point layer.
- `publish_weather`: a correlated normalized weather product or one safe
  unavailable code. Raw bodies, remote URLs, styles, scripts, and provider
  error text are not accepted.

The map contract contains an ID, title, bounded points, and provenance. Each
point contains a unique ID, a label, and valid WGS84 coordinates. Plugins cannot
provide MapLibre styles, JavaScript, markup, URLs, callbacks, or theme values.

## Enforced limits

| Boundary                         | Version-one limit |
| -------------------------------- | ----------------: |
| Frame payload                    |             1 MiB |
| Manifest                         |            64 KiB |
| Installed plugin folders scanned |               128 |
| Startup handshake                |         3 seconds |
| Map layers per plugin            |                16 |
| Points per map layer             |            10,000 |
| Layer/point identifiers          |          96 bytes |
| Layer title                      |         120 bytes |
| Point label                      |         200 bytes |
| Weather stations per request     |                10 |
| Model grid points per request    |               512 |
| Raster tiles per request         |                16 |
| Decoded bytes per raster tile    |           192 KiB |
| Decoded raster bytes per layer   |           640 KiB |
| Declared HTTPS origins           |                 8 |

Map coordinates must be finite and inside latitude `[-90, 90]` and longitude
`[-180, 180]`. Duplicate IDs and empty/control-character text are rejected.

## Python SDK proof

`sdk/python/wyrmgrid_sdk` uses only the Python standard library. The bundled
`Fleet Locations` plugin demonstrates stable snapshot consumption. Three
first-party providers demonstrate weather publication: Open-Meteo model grid,
AviationWeather.gov airport reports, and RainViewer PNG radar tiles. The SDK's
HTTPS helper allows only approved exact origins, rejects redirects, applies a
15-second timeout, checks content type, and reads no more than the requested
byte ceiling.

## Deferred hardening

Protocol and process separation are not an OS sandbox. Before unreviewed
community plugins are recommended, WyrmGrid needs signed packages, publisher
identity, tamper detection, CPU/memory/process quotas, message-rate limits,
restart throttling, OS-specific filesystem and network isolation, SDK
conformance suites, safe update/rollback, and a security review. Until then,
Forge labels the runtime a developer preview and users should run only code they
trust.

The proposal for supplying that distribution boundary through WyrmGrid Aerie
is documented separately in
[ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
and the
[hosted-platform implementation plan](../operations/hosted-platform.md). It does
not change plugin protocol version 1 or authorize community publication.

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
4. The host sends `hello` with the expected plugin ID and exact grants.
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
- `shutdown`: request for an orderly exit.

Snapshots never contain the OnAir API key, raw OnAir JSON, raw SimConnect
variables, FSUIPC offsets, provider paths, or native error text. Provider models
are translated into stable WyrmGrid domain summaries before this adapter.

## Plugin messages

- `ready`: exact plugin ID and supported plugin API version.
- `publish_map_layer`: one host-rendered point layer.

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

Map coordinates must be finite and inside latitude `[-90, 90]` and longitude
`[-180, 180]`. Duplicate IDs and empty/control-character text are rejected.

## Python SDK proof

`sdk/python/wyrmgrid_sdk` uses only the Python standard library. The bundled
`Fleet Locations` plugin demonstrates the public boundary by converting known
aircraft or current-airport coordinates into a calculated Atlas layer. It does
not claim that an aircraft is idle because the current stable snapshot does not
establish that fact.

## Deferred hardening

Protocol and process separation are not an OS sandbox. Before unreviewed
community plugins are recommended, WyrmGrid needs signed packages, publisher
identity, tamper detection, CPU/memory/process quotas, message-rate limits,
restart throttling, OS-specific filesystem and network isolation, SDK
conformance suites, safe update/rollback, and a security review. Until then,
Forge labels the runtime a developer preview and users should run only code they
trust.

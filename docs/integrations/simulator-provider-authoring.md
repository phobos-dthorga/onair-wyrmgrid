# Simulator provider authoring

WyrmGrid has two intentionally different extension seams:

| Extension          | Supplies                                 | Trust boundary                          | Contract                    |
| ------------------ | ---------------------------------------- | --------------------------------------- | --------------------------- |
| Simulator provider | Validated simulator facts to WyrmGrid    | Host-approved supervised native sidecar | WyrmGrid Bridge protocol v1 |
| Ordinary plugin    | Product features derived from host facts | Deny-by-default community process       | Plugin protocol v1          |

A provider talks to SimConnect, FSUIPC, or another simulator-local API and
translates it into WyrmGrid's stable model. A plugin can request a sanitized
copy of that model. A plugin cannot supply raw telemetry, register providers,
choose the active source, or inherit a provider's capabilities.

## Implemented contracts

- `providers/<provider>/provider.json` declares provider identity, version,
  supported platforms and simulators, executable, and capabilities.
- `schemas/simulator-provider-manifest-v1.schema.json` defines manifest version
  1. IDs use reverse-domain notation and entry points are safe relative paths.
- `schemas/bridge-protocol-envelope-v1.schema.json` defines Bridge protocol
  version 1.
- `schemas/simulator-telemetry-snapshot-v1.schema.json` defines the
  application-owned telemetry snapshot.
- `crates/bridge-protocol` is the Rust reference codec and validator.
- `schemas/fixtures/bridge-*-v1.json` and
  `schemas/fixtures/simulator-telemetry-v1.json` are sanitized compatibility
  fixtures.

Provider packages are not ordinary `plugin.json` packages. Version 1 registers
only explicitly host-approved providers; a folder copied into the plugin
directory is neither discovered nor executed as a simulator provider.

## Transport and lifecycle

Bridge uses standard input and standard output. Each message is a four-byte
unsigned big-endian JSON length followed by that many bytes. Frames are limited
to 64 KiB. Both directions use independent positive, strictly increasing
sequence numbers.

The startup sequence is:

1. WyrmGrid validates the manifest, platform, executable name, and absolute
   host-owned executable path.
2. The host starts the sidecar with piped standard I/O, no console window, and a
   scrubbed environment.
3. The host sends `hello` with its version, the expected provider ID, and the
   exact requested capabilities.
4. Within three seconds the provider returns `hello`. Its identity and version
   must match the manifest, its simulator must be declared, and its capabilities
   may not exceed the manifest or omit a requested capability.
5. The host sends `start_telemetry` with its maximum accepted frequency. The
   current application requests one hertz; protocol v1 permits at most ten.
6. The provider emits bounded `state` messages and telemetry snapshots. Envelope
   and snapshot sequences must advance independently.
7. The host sends `shutdown` and grants a short orderly-exit window before
   terminating the process. Application exit also cleans up the child.

The desktop renders these real transitions as a four-cue Bridge ritual: wake
the sidecar, verify identity, seal the read-only channel, and link the simulator.
It never delays startup to prolong the animation; a fast handshake may complete
the middle cues together. Motion is disabled when the user prefers reduced
motion.

Any malformed, oversized, repeated, out-of-order, wrong-source, undeclared, or
invalid message fails the provider. UI and diagnostics receive only stable state
codes such as `simconnect.waiting_for_simulator`; raw native error text is not
forwarded. An absent simulator is a normal `waiting_for_simulator` state, not an
application failure.

## Telemetry translation rules

Provider-specific names, offsets, handles, packing, units, and optional aircraft
extensions stay inside the provider. Emit only facts established by the source:

- preserve provider ID, revision, observation time, freshness, simulator family,
  and simulator version as provenance;
- use the units named by the snapshot fields;
- omit an optional field when the source cannot establish it safely;
- reject non-finite values and impossible coordinates before emission;
- keep sequence numbers monotonic for the lifetime of the provider process; and
- never encode a recommendation or inferred business state as a raw fact.

The host validates the snapshot again and confirms that its provenance provider
and simulator family match the approved manifest. The core exposes one selected
provider at a time. It does not average, fill, or silently fail over between
sources. The latest snapshot is publishable only while that provider is both
running and connected; disconnect, failure, stop, or unavailability immediately
withholds it from the interface and plugins.

## Adding an FSUIPC provider

FSUIPC should be a sibling package such as `providers/fsuipc`, not a mode in the
SimConnect executable. Its implementation should:

1. choose a distinct reverse-domain provider ID, executable, and manifest;
2. declare only `telemetry_read` until another capability has its own reviewed
   contract and explicit user action;
3. detect the installed FSUIPC generation, supported simulator, architecture,
   connection state, and any applicable licensing boundary;
4. keep offset numbers, byte layouts, conversions, and SDK/wrapper details
   private to the provider;
5. translate supported offsets into the same telemetry snapshot with FSUIPC
   provenance and omit unsupported facts;
6. add sanitized raw-to-domain fixtures, boundary tests, Bridge handshake tests,
   absent/incompatible/disconnect tests, and an outside-repository live matrix;
7. add the provider to the host's approved registration list and source picker;
   and
8. preserve the selected source if SimConnect is unavailable—never switch to
   FSUIPC without a visible user choice or a separately documented policy.

This makes FSUIPC interchangeable at the application boundary without claiming
that its offsets or behaviour are identical to SimConnect.

## Adding an ordinary telemetry-consuming plugin

Add `simulator_telemetry_read` to the plugin's requested permissions and handle
the `simulator_telemetry_snapshot` host message. The host sends the current
snapshot when one exists and later snapshots when their sequence advances. The
grant does not include historical tracks, raw SimConnect variables, FSUIPC
offsets, simulator commands, plan loading, provider selection, or arbitrary
local access.

The Python SDK offers an optional `on_simulator_telemetry` callback. Existing
plugins that do not request the permission receive no telemetry and require no
code change.

## Building and staging the first provider

The normal development launch prepares the sidecar before starting Tauri:

```powershell
npm run dev
```

`npm run provider:prepare` performs the same debug build and Tauri staging
without launching the desktop. Development discovery checks `target/debug` in
the workspace. Release preparation builds the provider in release mode and
copies it to Tauri's ignored `binaries` staging directory with the required
target-triple suffix. CI verifies the preparation command and Tauri bundles the
declared external binary only in Windows packages. Non-Windows builds skip this
native provider. A custom build can be selected with an absolute path:

```powershell
$env:WYRMGRID_SIMULATOR_PROVIDER_PATH = "C:\absolute\path\wyrmgrid-simconnect-provider.exe"
```

The SimConnect provider searches for `SimConnect.dll` beside its executable,
then an absolute `WYRMGRID_SIMCONNECT_DLL`, then `MSFS2024_SDK`, and finally the
standard `C:\MSFS 2024 SDK\SimConnect SDK\lib\SimConnect.dll` location. Only
the provider receives those approved path variables. Do not commit Microsoft
SDK files or copy them into application artifacts until redistribution terms
have been reviewed.

The current automated suite proves framing, validation, translation, handshake,
clean shutdown, and simulator-absent degradation. Before claiming live support,
run a sanitized outside-repository integration session against a supported MSFS
2024 build and representative aircraft, recording simulator/provider versions
and behavioural results without routes, coordinates, usernames, or identifiers.

## Community-provider release gate

The protocol is open for future providers, but loading unreviewed native
executables is not yet a supported end-user workflow. Public provider packages
need signed artifacts, publisher identity, hashes and tamper checks, canonical
installation roots, explicit permission/trust review, CPU/memory/process limits,
rate and restart throttling, safe update/rollback, vulnerability handling, and a
provider conformance kit. Until those controls ship, providers are built-in or
maintainer-approved development integrations only.

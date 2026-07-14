# ADR-0011: Core simulator capability through versioned provider sidecars

- Status: Accepted
- Date: 2026-07-15

## Context

Simulator context is important enough to be a first-class WyrmGrid capability,
but SimConnect, FSUIPC, and other native APIs have different availability,
licensing, architecture, failure, and compatibility constraints. Linking any of
those ABIs into the desktop process would let an optional integration destabilise
the application and would make community expansion depend on WyrmGrid's Rust or
operating-system ABI.

Ordinary WyrmGrid plugins are also the wrong boundary for acquiring raw
simulator data. They are product extensions that consume host-owned models.
Letting them open SimConnect or borrow provider credentials would bypass the
permission, provenance, validation, and privacy boundary.

## Decision

WyrmGrid owns simulator support as a core application capability while obtaining
simulator-specific data through separately supervised provider sidecars.

- The core owns provider discovery, process supervision, protocol validation,
  source selection, the stable telemetry model, and permission-filtered delivery
  to the interface and plugins.
- Each simulator integration declares a versioned `provider.json` and speaks the
  language-neutral WyrmGrid Bridge protocol over bounded framed standard I/O.
- The first-party MSFS 2024 SimConnect executable is distributed as a bundled
  provider for a coherent user experience, but still runs outside the desktop
  process and degrades safely when MSFS or SimConnect is absent.
- FSUIPC, MSFS 2020, X-Plane, and community integrations use separate providers.
  They do not become conditional branches in the interface or application
  domain.
- Version 1 permits one selected active telemetry provider. WyrmGrid never
  silently merges or switches between SimConnect, FSUIPC, or another source.
- Ordinary plugins may request `simulator_telemetry_read`. A granted plugin
  receives only the validated application-owned snapshot, never raw simulator
  variables, offsets, handles, paths, or provider errors.
- `telemetry_read`, `active_plan_read`, `flight_plan_load`, and
  `command_execute` are distinct capabilities. Read access never implies a
  simulator write.
- The Bridge protocol, provider-manifest schema, telemetry schema, plugin
  protocol, and application release remain independently versioned contracts.

Bundled providers are host-approved. Community provider installation is not a
publicly supported feature until package signing, publisher identity, hashes,
tamper detection, canonical installation paths, update/rollback, resource
limits, and an explicit user trust flow are implemented. An ordinary plugin
cannot register or choose a provider.

## Consequences

WyrmGrid gains a stable core telemetry experience and an extension seam that
does not expose Rust, C++, Tauri, or operating-system ABI compatibility. A
provider failure is containable, source provenance remains explicit, and
FSUIPC can be added without changing the consumer-facing telemetry contract.

The cost is another protocol and supervised process to test and package. The
provider executable is built and staged through the Tauri external-binary flow;
provider compatibility still requires fixtures plus outside-repository live
tests. Release packaging of the Microsoft SimConnect client remains subject to
an SDK redistribution review; Microsoft binaries are not committed to this
repository.

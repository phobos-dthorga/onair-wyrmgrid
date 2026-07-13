# ADR-0008: Provider adapters and canonical operational snapshots

- Status: Accepted
- Date: 2026-07-14

## Context

Dispatch and flight analysis need to combine OnAir jobs and aircraft, SimBrief
operational flight plans, SayIntentions.AI ATC and crew context, aviation
weather, online-network activity, simulator telemetry, navigation data, and
imported flight-plan files. These sources use
different identifiers, clocks, revisions, authentication methods, availability
rules, licences, and meanings for apparently similar fields.

Letting each feature consume provider payloads directly would create parallel
route and flight models, couple the interface and plugins to unstable external
schemas, and make stale or calculated values easy to misrepresent as current
OnAir facts.

## Decision

- Raw provider payloads remain private to cohesive Rust adapter crates. The
  initial adapters are OnAir, SimBrief, SayIntentions.AI, aviation weather,
  online networks, flight-plan files, and simulator sidecars.
- Provider adapters translate into application-owned, serializable snapshots.
  The central types are conceptually `FlightPlanSnapshot`, `RouteSnapshot`,
  `WeatherSnapshot`, `OnlineNetworkSnapshot`, `AtcServiceSessionSnapshot`, and
  `SimulatorSessionSnapshot`. Exact Rust types are introduced only with their
  first implemented use case.
- Snapshots are immutable observations. A refresh creates a new snapshot or
  revision rather than silently changing the historical meaning of an existing
  one.
- Every material field or coherent field group carries provenance: category,
  provider, provider revision or AIRAC when relevant, generated/effective time,
  retrieval time, transformation version, and freshness state. Provider record
  identifiers are retained only when required for refresh or correlation.
- Similar values are not silently reconciled. Conflicting origin, destination,
  aircraft, payload, route, time, or fuel values remain distinguishable until an
  application service applies an explicit, explainable rule.
- Application services own orchestration and reconciliation. Tauri commands are
  thin, and Svelte handlers only request actions and render results.
- Each provider declares narrow capabilities such as import, refresh, generate,
  open externally, export, load into simulator, or read telemetry. Read access
  does not imply authority to perform an external write or simulator mutation.
- External account identifiers, tokens, application keys, and redirect state do
  not become domain-model fields. User tokens belong in the operating-system
  credential store. Shared application secrets must not be embedded in the
  desktop binary; a hosted component requires a separate accepted decision and
  threat-model review.
- Provider failure degrades independently. Cached snapshots remain available
  with visible age and source, and no optional provider is required for normal
  startup or offline use.
- Community plugins receive only versioned, sanitized host models through
  separately approved capabilities. They never receive provider credentials,
  raw payloads, or a generic proxy to provider endpoints.
- Imported routes and downloaded provider content are untrusted input and pass
  through size limits, schema validation, bounded parsing, and identifier
  normalization before persistence or display.

## Initial operational model

The first `FlightPlanSnapshot` must be able to represent:

- origin, destination, alternates, and planned times;
- aircraft type and an optional locally correlated fleet-aircraft reference;
- payload, weight, fuel, endurance, and units without inventing missing values;
- an ordered route with source text and normalized legs when available;
- AIRAC or dataset revision and plan-generation time;
- provider-specific plan correlation kept behind an opaque reference;
- provenance and freshness for every imported or calculated group.

It must not promise take-off or landing performance, regulatory dispatch
validity, or real-world navigational fitness merely because a provider included
related data.

## Consequences

WyrmGrid can compare planned and actual operations without making SimBrief,
SayIntentions.AI, Navigraph, a network, or a simulator part of its core ABI.
Adapters can change or disappear while stored snapshots and application rules
remain stable.

The translation and reconciliation layers require fixtures and explicit mapping
tests. Some apparent duplication is intentional where providers assign
different meanings to similar fields. A universal provider framework or a
single catch-all integrations crate is not justified; shared abstractions are
extracted only after at least two implemented adapters need the same contract.

The implementation programme is documented in the
[integration overview](../../integrations/README.md).

# WyrmGrid Bridge and simulator providers

WyrmGrid Bridge is the out-of-process boundary between the desktop application
and native or simulator-local APIs. MSFS 2024 is the primary target. MSFS 2020,
FSUIPC, and X-Plane are separate providers behind the same versioned Bridge
protocol, not conditional code paths inside Svelte or the application domain.

[SayIntentions.AI](sayintentions.md) is a separate account provider. Its local
`flight.json` and SAPI describe the AI ATC session, communications, and
SayIntentions-owned airport state; they do not replace Bridge telemetry. The
application correlates their sanitized snapshots by explicit flight references,
callsign, airports, and time without sending raw Bridge values or provider
credentials across either boundary.

## Provider priority

1. **MSFS 2024 through SimConnect**: primary supported simulator and first
   complete plan-to-actual loop.
2. **MSFS 2020 through SimConnect**: compatibility where verified semantics are
   shared; differences remain explicit capabilities.
3. **FSUIPC**: optional provider for users and aircraft that expose useful data
   through it. It is not a silent substitute for failed SimConnect access.
4. **X-Plane 12 Web API**: second simulator family, using its localhost REST and
   WebSocket boundary.
5. Other simulators only after a concrete user need and supported integration
   contract exist.

## Process and protocol boundary

- Each native integration runs in a separately supervised sidecar and may crash,
  hang, be absent, or be incompatible without taking down WyrmGrid.
- The sidecar and host begin with a versioned handshake describing protocol
  version, provider, simulator version, architecture, connection state, and
  narrow capabilities.
- Capabilities distinguish telemetry read, active-plan read, flight-plan load,
  command execution, and provider-specific extensions. Telemetry read does not
  grant simulator mutation.
- The host validates framing, message size, frequency, enum values, timestamps,
  units, coordinates, monotonic sequence, and capability use.
- Simulator-specific identifiers and raw values are translated inside the
  provider. The host receives stable snapshots and lifecycle events.
- The host applies timeouts and restart limits. It never loops indefinitely when
  a simulator or sidecar is unavailable.
- The Bridge binds only to the local mechanism required by its provider and does
  not expose a remotely reachable control service by default.

The Bridge protocol requires its own fixtures, validation tests, documentation,
and compatibility decision before the first executable sidecar ships.

## MSFS 2024 slice

### Phase 1: detection and read-only telemetry

- Detect SimConnect availability, simulator version, active aircraft, and
  connection transitions.
- Translate a minimal, verified set of standard simulation variables: position,
  altitude, attitude, ground and air speed, on-ground state, UTC simulation time,
  fuel quantities with units, gross weight, engine-running state, parking brake,
  and pause or simulation-rate state where supported.
- Emit bounded snapshots at an application-appropriate rate, initially no more
  than once per second plus significant lifecycle transitions. The sidecar may
  sample internally at a different rate without flooding the host.
- Treat third-party-aircraft variables as optional extensions with explicit
  provider and aircraft-profile versions.

### Phase 2: flight lifecycle and plan comparison

- Derive candidate block-off, take-off, landing, and block-on events through a
  tested state machine in a shared Rust service, not interface handlers.
- Let the user confirm or correct ambiguous lifecycle events.
- Correlate the session to a selected `FlightPlanSnapshot` without embedding
  SimBrief or OnAir payloads in Bridge messages.
- Store bounded track and phase summaries with retention controls rather than
  every high-frequency sample forever.

### Phase 3: explicit flight-plan load

MSFS 2024's SimConnect flight API documents loading an existing flight-plan
file. WyrmGrid may export a validated `.pln` and ask the sidecar to load it only
after an explicit user action, a negotiated `flight_plan_load` capability, and a
clear result. It must preserve the existing simulator state on failure and must
not claim that every aircraft flight-management system accepted the route.

No simulator command writes to OnAir. Any future simulator mutation beyond plan
load needs its own user-facing action, capability, tests, and safety review.

## MSFS 2020 and FSUIPC

Shared SimConnect request definitions may live in a private Bridge library when
fixtures prove identical behaviour. Simulator-version checks remain explicit;
MSFS 2024 success is not evidence of MSFS 2020 compatibility or vice versa.

FSUIPC is a separate provider with separate discovery, availability, version,
licensing, offset definitions, and unit tests. If SimConnect and FSUIPC are both
available, the user or a documented provider policy selects the source. Values
are not merged silently.

## X-Plane 12 provider

X-Plane 12.1.1 and later includes a local HTTP and WebSocket API for enumerating,
reading, writing, and streaming datarefs; later API versions add commands and
flight initialization. Users can disable incoming traffic in X-Plane.

- Discover supported API versions before using an endpoint.
- Resolve datarefs by name for each simulator session and do not persist numeric
  IDs across launches.
- Subscribe only to a reviewed allowlist and bound message frequency and size.
- Begin read-only even where the API exposes writable datarefs and commands.
- Represent disabled, unavailable, unsupported-version, and permission-denied
  states without repeated connection errors.
- Keep the Web API behind an X-Plane Bridge provider so local protocol details
  do not enter application or plugin contracts.

## Security, privacy, and observability

- Simulator telemetry, routes, coordinates, aircraft identifiers, usernames,
  local paths, and raw provider errors are excluded from Sentry.
- Sidecar executable paths and arguments are host-owned and never accepted from
  untrusted plugin messages.
- Community plugins need the existing `simulator_telemetry_read` capability and
  receive only the bounded, versioned host model. Plan loading or commands need
  separate deny-by-default capabilities if ever exposed.
- Local sockets, ports, and peer processes are authenticated or restricted as
  strongly as each provider permits; origin and process assumptions are covered
  in the threat model.
- Sidecar diagnostics use stable codes and aggregate counts rather than raw
  simulation values.

## Required validation

- protocol handshake, version negotiation, framing, limit, timeout, restart, and
  capability-denial fixtures;
- deterministic lifecycle state-machine tests including pause, slew, replay,
  disconnect, crash, diversion, touch-and-go, and simulator-rate changes;
- unit and fuel-total tests across representative default and third-party
  aircraft, without claiming universal aircraft compatibility;
- simulator-absent and optional-provider degradation tests on every platform;
- `.pln` generation and explicit load success/failure tests against a supported
  MSFS 2024 installation outside the repository; and
- X-Plane API-version, disabled-server, dataref-resolution, and reconnect tests.

## References

- [MSFS 2024 SimConnect flight APIs](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/API_Reference/Flights/Flights.htm)
- [MSFS 2024 SimConnect SDK](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/SimConnect_SDK.htm)
- [X-Plane local Web API](https://developer.x-plane.com/article/x-plane-web-api/)
- [X-Plane data access API](https://developer.x-plane.com/sdk/XPLMDataAccess/)

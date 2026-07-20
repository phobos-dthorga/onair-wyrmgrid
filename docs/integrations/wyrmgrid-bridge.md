# WyrmGrid Bridge and simulator providers

WyrmGrid Bridge is the out-of-process boundary between the desktop application
and native or simulator-local APIs. MSFS 2024 is the primary target. MSFS 2020,
FSUIPC, and X-Plane are separate providers behind the same versioned Bridge
protocol, not conditional code paths inside Svelte or the application domain.

## Implementation status

The version-one foundation is implemented: manifest and Bridge schemas,
length-prefixed protocol codec, sanitized fixtures, stable telemetry domain
model, multi-registration supervisor with one selected active provider, the
MSFS 2024 SimConnect provider executable, desktop connection controls, and
permission-filtered telemetry delivery to ordinary plugins. The provider is
compiled and exercised by automated handshake, translation, validation,
shutdown, and unavailable-simulator tests.

Live MSFS 2024 behaviour has not yet been certified. A sanitized
outside-repository session with the simulator and representative aircraft is
still required before the project describes the provider as live-supported.
The WyrmGrid provider executable is staged as a Windows-only declared Tauri
external binary. Bundling the Microsoft SimConnect client remains pending
review of the SDK redistribution terms; no Microsoft binary is committed here.

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

The Bridge protocol has independent fixtures, validation tests, documentation,
and the compatibility decision in
[ADR-0011](../architecture/decisions/0011-core-simulator-capability-provider-sidecars.md).
See [simulator provider authoring](simulator-provider-authoring.md) for the
implemented contract and the FSUIPC/community-provider path.

## Adjacent audio-recording boundary

Simulator-synchronised audio is not a Bridge protocol version 1 payload.
Continuous PCM or encoded media would violate the purpose and limits of the 64
KiB JSON telemetry channel and couple audio backpressure to simulator facts.
Bridge providers continue to supply attributed radio state and session evidence.
The separate Audio Capture Provider protocol version-one foundation now defines
capability-labelled sources, bounded control headers, and separately framed
encoded-packet bodies, but no native capture or user-facing availability exists.

MSFS 2024's documented COM facts do not establish isolated COM audio. X-Plane
12's named audio groups make isolated capture a feasibility candidate, but its
planned Web API Bridge provider remains the telemetry authority. Any future
in-process X-Plane audio tap must be thin, first-party, non-blocking, and unable
to make WyrmGrid policy or storage decisions. The complete planned boundary is
defined in
[ADR-0017](../architecture/decisions/0017-simulator-synchronised-audio-recording.md)
and the [simulator-audio plan](simulator-audio-recording.md).
The implemented wire contract is documented separately in the
[Audio Capture Provider protocol reference](audio-capture-provider-protocol.md).

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
- Bridge messages, diagnostics, Sentry, and ordinary plugins never receive
  microphone samples, communications audio, device labels, application labels,
  encoded-media paths, or audio content.
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

- [Simulator provider authoring](simulator-provider-authoring.md)
- [Simulator-synchronised audio recording](simulator-audio-recording.md)
- [MSFS 2024 SimConnect flight APIs](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/API_Reference/Flights/Flights.htm)
- [MSFS 2024 SimConnect SDK](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/SimConnect/SimConnect_SDK.htm)
- [X-Plane local Web API](https://developer.x-plane.com/article/x-plane-web-api/)
- [X-Plane data access API](https://developer.x-plane.com/sdk/XPLMDataAccess/)

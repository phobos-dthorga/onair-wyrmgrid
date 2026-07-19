# SayIntentions.AI integration plan

SayIntentions.AI is an optional, account-gated first-party provider for AI ATC,
crew, ACARS, airport-operation, and active-flight context. It complements
WyrmGrid Bridge rather than replacing it: Bridge remains the authoritative
adapter for simulator actuals, while SayIntentions supplies facts and actions
owned by its AI world.

The cross-provider order, capability-level delivery steps, and acceptance gates
for the selected high-value work live in the
[high-value provider integration process](high-value-provider-process.md). This
document remains authoritative for the SayIntentions contract and
provider-specific boundaries.

The maintainer's 2026-07-19 screenshots of the public provider documentation,
the reconciled endpoint inventory, field-disposition decisions, SimAPI notes,
and immediate-use conclusion are preserved in the
[dated public-contract observation](sayintentions-contract-observation-2026-07-19.md).

## Verified public contract

As documented on 2026-07-19:

- SAPI is a preview REST API for communications, weather and live data, airport
  operations, flight management, and virtual-airline integration;
- a normal subscribed pilot can obtain the SAPI key from the Pilot Portal or the
  local active-flight payload; demo accounts cannot use the API;
- the same active-flight payload is available through the local
  `http://localhost:63287/flightJSON` endpoint or the generated
  `%LOCALAPPDATA%\SayIntentionsAI\flight.json` file, is updated approximately
  every 3 to 30 seconds, and must be treated as read-only;
- the payload contains credentials and identity together with callsign,
  flight-plan, runway, gate, traffic, and other active-flight fields;
- API operations use an account key, and some require an active flight session;
- voice-generation requests must remain aviation-themed and family-friendly,
  are limited in message length, incur provider cost, and may be restricted if
  abused; and
- SAPI's preview availability and pricing may change, while the provider states
  that `flight.json` and SimAPI will remain open.

This means the user's existing subscribed account is sufficient for initial
authenticated testing. It is not a WyrmGrid developer credential and must never
be shared, committed, copied into fixtures, or entered into the Svelte interface
unless a reviewed secret-entry flow is implemented.

No partnership is documented as necessary for these ordinary pilot-facing
surfaces. That does not mean WyrmGrid can use them today: the local adapter,
secret boundary, synthetic fixtures, and outside-repository authenticated
compatibility test still have to be implemented. VA-Link remains the exception
because its persistent import requires a separate provider-issued virtual-
airline key.

## Product boundary

WyrmGrid initially uses two SayIntentions surfaces:

1. **Local active-flight read adapter** for presence, active-flight identity,
   and a minimal allowlist of SayIntentions-owned state. A bounded spike chooses
   between the fixed loopback HTTP endpoint and the documented `flight.json`
   file before the released transport is fixed.
2. **SAPI adapter** for carefully selected reads and explicit user-initiated
   communications or airport actions.

WyrmGrid does not implement SayIntentions SimAPI merely to impersonate a flight
simulator. SayIntentions already supports MSFS 2020/2024 and X-Plane directly;
the WyrmGrid Bridge reads those simulators for WyrmGrid's own plan-versus-actual
analysis. A SimAPI adapter is considered only for a future simulator that lacks
native SayIntentions support and has a concrete tested need.

SayIntentions also imports SimBrief internally for some products. WyrmGrid keeps
its own SimBrief snapshot and compares origin, destination, callsign, and plan
identity where safely available. It never assumes both products imported the
same OFP or silently replaces one plan with the other.

## Phase 1: local read-only connection

- Keep the integration disabled until the user enables it.
- Spike both documented transports: a request to the fixed loopback endpoint and
  a read of the documented `flight.json` path on Windows. Do not use a LAN
  address, trust a hostname from the payload, or scan unrelated directories or
  simulator installations.
- Ask before reading either transport because the payload contains the user's
  email, account ID, API key, route, coordinates, callsign, and group
  configuration.
- Parse a bounded snapshot in Rust, tolerate an unavailable endpoint, empty
  inactive-flight response, absent file, and partial rewrite, and debounce or
  poll no faster than the documented update interval.
- Select the released transport only after comparing local-server exposure,
  filesystem permissions, startup ordering, cancellation, and supportability.
- Treat the API key as a `SecretString` immediately. Never include the raw
  payload, access path, or secret-bearing parse errors in logs, UI state, SQLite,
  Sentry, or plugin messages.
- Use the discovered key only for the active process by default. Offer opt-in
  persistence only after the operating-system credential store exists.
- Translate only implemented fields into a provider-labelled
  `SayIntentionsSessionSnapshot`; discard email, user ID, host logging settings,
  multiplayer group codes, and unrelated configuration.
- Show unavailable, inactive-flight, malformed, unsupported, and permission
  states without turning expected absence into an error alert.

The initial snapshot may include an opaque flight reference, callsign, origin,
destination, assigned gate or runway, current airport, provider update time, and
connection state when fixtures verify those fields. Simulator position, fuel,
weight, and lifecycle actuals still come from WyrmGrid Bridge.

## Phase 2: narrow read-only SAPI

Begin with low-risk, user-visible reads:

- current frequencies;
- current parking or gate assignment;
- airport information required by the active WyrmGrid plan; and
- bounded session status required to explain whether a requested action is
  available.

Communication history is excluded initially because transcripts may contain
callsigns, routes, clearances, coordinates, generated audio links, and other
free-form operational or personal data. SayIntentions weather and VATSIM
endpoints also remain separate from WyrmGrid's primary AviationWeather.gov and
VATSIM adapters; if used later, their provider and observation time remain
explicit rather than being silently merged.

The Rust adapter pins the documented HTTPS origin. It must not trust a hostname
read from `flight.json`, follow an unexpected cross-origin redirect, or expose a
generic SAPI request proxy.

## Phase 3: explicit communications and airport actions

The first write candidates are:

- send a user-previewed ACARS/CPDLC/telex message into the active flight;
- ask a selected crew or intercom entity to make a short operational
  announcement; and
- request a user-selected gate assignment.

Messages use host-owned aviation templates populated from a selected operational
snapshot. They are length checked, family-friendly, shown to the user before
sending, and never derived directly from untrusted plugin or provider text.
Automatic rephrasing is off until the user explicitly selects it.

Every send is non-idempotent: it has no automatic retry after an ambiguous
timeout, receives a per-operation cooldown, and records only a local sanitized
result. WyrmGrid starts with no automatic voice generation. Any later automation
requires a named user-enabled rule, duplicate suppression, and a hard per-flight
budget comfortably below the provider's “few dozen” guidance.

Frequency mutation, arbitrary `setVar`, pause control, raw ATC simulation, and
unbounded communication injection are excluded from the initial integration.
Each needs a distinct capability, explicit user action, validation, and safety
review before adoption.

## Phase 4: dispatch and virtual-airline collaboration

Potential later uses include:

- WyrmGrid Dispatch messages delivered through ACARS or a dispatcher persona;
- arrival, delay, diversion, maintenance, or job-deadline notifications;
- planned-versus-cleared route and assigned-gate comparisons;
- crew announcements based on confirmed flight lifecycle events; and
- optional VA-Link content for an OnAir company or community airline.

VA-Link is not enabled by an ordinary pilot key alone. Its import operation uses
a separate virtual-airline key supplied by SayIntentions and stores submitted
crew, dispatcher, copilot, or SkyOps material for future flights. WyrmGrid must
contact SayIntentions, define ownership and deletion behaviour, review every
submitted field, and obtain explicit user or airline-administrator approval
before implementing it.

## Credential and privacy controls

- The API key is passed as a query parameter by the current SAPI contract. HTTP
  request URLs, redirect locations, proxy diagnostics, and `reqwest` errors can
  therefore become secret-bearing and must be mapped to bounded codes before
  logging or telemetry.
- Never enable HTTP wire logging for SAPI or place a complete request URL in an
  error, breadcrumb, support bundle, test snapshot, or crash report.
- Pin the official API origin and HTTPS scheme; reject origin changes rather than
  forwarding the account key.
- Do not persist the raw local active-flight payload, communications, audio URLs,
  routes, coordinates, callsigns, user IDs, email addresses, or multiplayer
  settings.
- Plugins receive neither the key nor a generic SayIntentions capability. A
  future read or send capability must be operation-specific, deny-by-default,
  bounded, and mediated by host templates and user confirmation.
- Removing or disabling the connection drops the in-memory key and all
  unpublished session state.

## Required validation

- synthetic local-HTTP and `flight.json` fixtures for active, empty, absent,
  partial-write, malformed, oversized, unknown-field, and schema-change cases;
- fixed-loopback, LAN-address rejection, unavailable-port, cancellation, and
  transport-fallback tests before choosing or supporting fallback behaviour;
- canary tests proving that API keys, email, user ID, path, host, callsign, route,
  coordinates, and communication text cannot enter logs, errors, Sentry, SQLite,
  or plugin models;
- pinned-origin, redirect, timeout, TLS, offline, cancellation, and secret-bearing
  query tests;
- no-retry, duplicate-suppression, message-length, template, cooldown, and
  per-flight-budget tests for writes;
- side-by-side correlation tests where SayIntentions and WyrmGrid have different
  SimBrief plans or simulator state; and
- an authenticated outside-repository test using the maintainer's ordinary
  subscribed account before claiming live compatibility. Only sanitized field
  mappings and results may be recorded; the key and captured raw response remain
  outside the repository.

Because SAPI is currently preview, every release that enables it must recheck the
official documentation, availability, pricing status, authentication contract,
and content rules.

## References

- [SayIntentions SAPI, flight.json, and SimAPI documentation](https://p2.sayintentions.ai/p2/docs/)
- [SayAs API](https://kb.sayintentions.ai/article/sayas-api)
- [VA-Link virtual-airline integration](https://kb.sayintentions.ai/article/va-link-virtual-airline-integration-api)
- [SayIntentions SimAPI announcement](https://www.sayintentions.ai/blog?p=support-any-flight-sim-with-our-new-sim-api-meanwhile-help-us-expand-our-ai-traffic)
- [SayIntentions built-in SimBrief context](https://kb.sayintentions.ai/article/entourage-faq)

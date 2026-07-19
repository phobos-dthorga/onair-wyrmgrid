# SayIntentions.AI public-contract observation — 2026-07-19

This record turns two maintainer-supplied screenshots of the public
SayIntentions.AI API documentation into a bounded WyrmGrid integration
reference. It was checked against the provider's current public documentation
on 2026-07-19.

It is **public-document evidence**, not a live provider response, partnership,
compatibility certification, or permission to expose a pilot's credentials.
The screenshots contain documentation examples rather than the maintainer's
live flight data. They were not copied into the repository.

## Immediate-use conclusion

A normal subscribed pilot can begin using the documented pilot-facing surfaces
without a WyrmGrid partnership:

- obtain the pilot SAPI key from the Pilot Portal or the local active-flight
  payload;
- read the active-flight payload through the local HTTP endpoint or generated
  file while SayIntentions is running; and
- call the preview SAPI endpoints allowed by that account and, where required,
  an active flight session.

Demo accounts cannot use SAPI. The separate VA-Link import is not unlocked by
the ordinary pilot key: it also requires a virtual-airline API key issued by
SayIntentions support. Nothing in the public document establishes that
WyrmGrid already implements or has live-tested any of these surfaces.

## Evidence register

| Evidence                |   Dimensions | SHA-256                                                            | Treatment                                          |
| ----------------------- | -----------: | ------------------------------------------------------------------ | -------------------------------------------------- |
| Maintainer screenshot 1 | 1728 × 14466 | `fddbaec24bf08c468cd42405d872676c716b524edc16e978f72f3039cc1b5dcc` | Visually inspected; not retained in the repository |
| Maintainer screenshot 2 | 2477 × 10085 | `00b569fa0f4a50306aebd7fea740d19e973ec01f4fe2c1e544d124bc0b45c31c` | Visually inspected; not retained in the repository |
| Current public HTML     |          n/a | n/a                                                                | Reconciled with the screenshots on 2026-07-19      |

The two screenshots overlap and together show the SAPI, `flight.json`, SimAPI,
LVAR/DataRef, limitation, and support sections of the same documentation page.
The HTML was used for exact names and values where the screenshots' downscaled
text was difficult to read. No credential, personal payload, or authenticated
provider response was used.

## Public availability and authentication

The observed documentation says:

- SAPI is in preview and currently free, with future availability or pricing
  subject to change;
- `flight.json` and SimAPI are intended to remain free and open;
- SAPI uses the base URL `https://apipri.sayintentions.ai/sapi/`;
- the normal SAPI key is visible in the Pilot Portal and is also included in the
  local active-flight payload;
- endpoint requirements are expressed as API key only or API key plus active
  session; and
- messages must be aviation-themed and family-friendly.

The current contract places the pilot key in a query parameter. A complete SAPI
URL is therefore secret-bearing and must never reach logs, history, analytics,
crash reports, Sentry, plugins, screenshots, or fixtures.

## SAPI endpoint inventory and WyrmGrid disposition

The provider describes all ordinary endpoints as HTTPS GET operations, except
`importVAData`, which is POST. HTTP method does not determine product safety:
several GET endpoints mutate provider or simulator state.

| Endpoint                | Documented access                      | Effect                                                                                            | WyrmGrid disposition                                                                    |
| ----------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `sayAs`                 | API key + session                      | Generates pilot, ATC, intercom, or inbound ACARS communication                                    | Later explicit, previewed send; no automatic retry after an ambiguous result            |
| `getCommsHistory`       | API key                                | Returns flight transmissions, coordinates, frequencies, messages, audio URLs, and mission context | Defer; high privacy and retention surface                                               |
| `getWX`                 | API key                                | Returns ATIS, METAR, TAF, active runway, wind, and optional frequencies                           | Defer as a secondary sourced fact; do not replace WyrmGrid's weather providers silently |
| `getTFRs`               | API key                                | Returns TFR GeoJSON                                                                               | Defer pending authority, coverage, freshness, and direct-source comparison              |
| `getVATSIM`             | API key                                | Returns VATSIM pilot/controller GeoJSON                                                           | Do not use as the primary feed; WyrmGrid has a direct VATSIM path                       |
| `assignGate`            | API key + session                      | Requests a named gate at a 3–4-character airport                                                  | Later explicit user action with airport and gate validation                             |
| `getParking`            | API key + session                      | Returns current parking identity and geometry                                                     | Early read candidate after sanitized fixture certification                              |
| `getAirport`            | API key + session                      | Returns airport data for airports in the current flight plan                                      | Early read candidate; preserve provider provenance                                      |
| `setFreq`               | API key + session                      | Changes active or standby COM1/COM2 frequency                                                     | Exclude initially; simulator mutation needs its own reviewed capability                 |
| `getCurrentFrequencies` | API key                                | Returns the current frequency configuration                                                       | Early read candidate after response-shape certification                                 |
| `setVar`                | API key + session                      | Sets an arbitrary simulator or system variable                                                    | Exclude; too broad for a provider adapter or plugin capability                          |
| `setPause`              | API key + session                      | Pauses or resumes the ATC simulation                                                              | Exclude initially; separate explicit action if a concrete need appears                  |
| `importVAData`          | Pilot API key + provider-issued VA key | Persists crew, dispatcher, copilot, or SkyOps material for future flights                         | Partnership/support-key phase only, with administrator approval and deletion semantics  |

For `sayAs`, the observed channels include COM1/COM2, three intercom channels,
corresponding `_IN` channels, and `ACARS_IN`. Messages are limited to 255
characters, or 128 for ACARS. AI rephrasing applies only to inbound channels and
is off by default in the contract. WyrmGrid should keep rephrasing off until the
user selects it for a reviewed message.

The provider currently states that there are no strict rate limits, while
warning that abusive access can be restricted. Its guidance is a few dozen
voice requests per flight, no rapid-fire calls, suitable delays, and caching of
weather or static data. WyrmGrid must impose its own lower budgets, cooldowns,
deduplication, and bounded polling rather than treating the absence of a strict
server limit as capacity.

The generic documentation recommends retrying transient failures and logging
responses. Those generic examples do not override WyrmGrid's safety rules:
non-idempotent sends receive no automatic retry after an ambiguous outcome, and
raw responses or secret-bearing URLs are never logged.

## Local active-flight transport

The same active-flight JSON is exposed through:

- `GET http://localhost:63287/flightJSON`, recommended by the provider for
  companion applications; and
- `%LOCALAPPDATA%\SayIntentionsAI\flight.json`, present during active flights.

The HTTP endpoint is documented as active while the Windows client is running,
returns `{}` when there is no active flight, and updates approximately every
3–30 seconds during a flight. The path is case-insensitive. The endpoint is also
documented as reachable on the local network and returns
`Access-Control-Allow-Origin: *`.

That convenience is a security boundary: an unauthenticated LAN-readable,
wildcard-CORS payload may contain an email address, user ID, API key, route,
coordinates, callsign, and multiplayer configuration. WyrmGrid must ask before
reading it, connect only to the fixed loopback address, never discover or expose
it over LAN, extract the key directly into secret memory, discard unneeded
fields, and never persist the raw JSON.

The released adapter should choose one transport only after a bounded spike
compares permissions, partial file writes, startup ordering, cancellation, and
local-server exposure. Provider preference alone is not sufficient to make the
HTTP transport safe.

## `flight_details` field decision record

The screenshots and current HTML document the following field groups. These are
candidate mappings, not proof of live field presence or semantics.

| Group                       | Documented fields                                                                                              | Initial handling                                                                                                               |
| --------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Authentication and identity | `Email`, `userid`, `flight_id`, `api_key`                                                                      | Keep only an opaque flight reference; extract the key into process-only secret memory; discard email and user ID               |
| System configuration        | `hostname`, log levels, `skynet_endpoint`, `enable_skynet`, `skynet_group_code`                                | Discard; do not trust provider-supplied origins or expose multiplayer configuration                                            |
| Aircraft and callsign       | `callsign`, `callsign_icao`                                                                                    | Allowlist as SayIntentions-owned correlation facts after fixture validation                                                    |
| Flight plan and route       | origin, destination, their coordinates, route, SID, STAR, departure/arrival runways, candidate arrival runways | Initially allowlist identifiers and assigned procedures/runways; defer route and coordinates unless a named feature needs them |
| Ground operations           | assigned gate and coordinates, taxi path, taxi object                                                          | Allowlist gate identifier; defer geometry and taxi guidance                                                                    |
| Clearances and operations   | cleared-for-landing runway, cleared-for-takeoff runway, distance to runway                                     | Candidate provider-labelled context; never replace Bridge actuals or infer a real-world clearance                              |
| Traffic and environment     | enablement, density, radius, aircraft ceilings, GA traffic, dispatcher log level                               | Discard initially; WyrmGrid should not mirror provider configuration accidentally                                              |
| Airport and weather         | ATIS airports, current airport, pattern direction, runway                                                      | Allowlist current airport and assigned runway; defer the rest pending a concrete sourced display                               |

WyrmGrid Bridge remains authoritative for simulator position, fuel, weight,
registration, and lifecycle actuals. Conflicts are displayed as separately
sourced facts; they are not silently merged.

## SimAPI and cockpit reference

SimAPI is documented as a Windows 10/11, language-agnostic, two-way file
adapter for integrating a simulator with SayIntentions:

- an adapter writes telemetry and radio-stack input to
  `%LOCALAPPDATA%\SayIntentionsAI\simAPI_input.json` approximately every
  0.75–1 second;
- the required metadata fields are `name`, `version`, `adapter_version`,
  `simapi_version`, and `exe`;
- optional SayIntentions-to-simulator changes arrive as JSONL in
  `%LOCALAPPDATA%\SayIntentionsAI\simAPI_output.jsonl`; and
- the adapter reads each line, applies the requested variable change if safely
  supported, and then clears or deletes the output file.

The page also lists paired MSFS LVARs and X-Plane `siai/` DataRefs for radio and
intercom transmit/receive state, ATC enablement, copilot control, audio volumes,
tuned station type, flight phase, takeoff/landing flags, taxi-path visibility,
and mechanic/cabin-crew call buttons. Settable variables are commands, not
ordinary observations, and require explicit authorization and validation.

The documented SimAPI limitations include no 3D-object injection, pushback or
ramp control, traffic injection or awareness, lighted taxi paths, or mission
scene objects. Rescue and fire missions operate without smoke, incident
objects, flares, or fires; cargo, passenger, training, and custom mission types
are described as otherwise supported.

WyrmGrid should not implement SimAPI for the current MSFS or X-Plane plan.
SayIntentions already has native integrations for those simulators, while
WyrmGrid Bridge owns WyrmGrid telemetry. SimAPI becomes relevant only when a
future simulator lacks native SayIntentions support, and then it belongs in a
separate sidecar with an explicit protocol version, command allowlist, atomic
file handling, and safe degradation.

## Unknowns that require sanitized certification

Before WyrmGrid claims live compatibility, an authenticated test performed
outside the repository must establish:

- actual response shapes, types, optional fields, unknown fields, and maximum
  payload sizes for the chosen reads;
- whether API-key-only operations behave sensibly without an active session;
- inactive-flight, startup, shutdown, stale-update, partial-file, and local HTTP
  failure behaviour;
- redirect, timeout, status-code, content-type, and malformed-response handling;
- whether the local HTTP server binds beyond loopback in the installed client
  and what host firewall controls apply;
- the exact meaning and lifecycle of returned flight, gate, runway, clearance,
  and frequency fields; and
- provider support for deletion or reset of persisted VA-Link material before
  that phase is considered.

Only a sanitized field-and-result report may enter the repository. The API key,
raw active-flight JSON, raw SAPI responses, communications, and identifying
values stay outside it.

## Recommended first implementation slice

1. Add a disabled-by-default SayIntentions connection setting with clear consent
   for reading the secret-bearing local payload.
2. Implement a Rust-only loopback/file transport spike and select one release
   transport after the security comparison.
3. Parse a strict, size-bounded allowlist into a provider-labelled session
   snapshot and prove by canary tests that discarded fields cannot escape.
4. Perform the outside-repository authenticated certification and create only
   synthetic sanitized fixtures from the confirmed schema.
5. Add `getCurrentFrequencies`, `getParking`, and `getAirport` as narrow reads.
6. Only then design one explicit, previewed `sayAs` ACARS or crew action with a
   hard budget, cooldown, and no ambiguous retry.

## References

- [SayIntentions SAPI, flight.json, and SimAPI documentation](https://p2.sayintentions.ai/p2/docs/)
- [WyrmGrid SayIntentions integration plan](sayintentions.md)
- [High-value provider integration process](high-value-provider-process.md)

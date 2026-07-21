# External integrations programme

WyrmGrid combines optional external services around a local-first application.
No integration is an availability dependency, and no external value loses its
source, age, or category when it enters the application.

## Operational flow

```text
OnAir jobs and fleet ------+
SimBrief OFP --------------+
Weather and nav data ------+--> operational snapshots --> Dispatch and Atlas
SayIntentions.AI ----------+              |                       |
                                          |                simulator-ready plan
Online networks --------------------------+--> Atlas overlays     |
                                                                  v
                                                        WyrmGrid Bridge
                                                                  |
                                                +-----------------+----------------+
                                                |                                  |
                                           MSFS 2024                          X-Plane 12
                                                |
                                                v
                                         actual-flight snapshot
                                                |
                                                v
                                      planned-versus-actual analysis
```

## Planned integrations

| Area               | Initial target                              | Product position                                    | Account or approval                                       |
| ------------------ | ------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- |
| Flight planning    | SimBrief latest OFP                         | First-party core adapter                            | SimBrief account; generation needs developer approval     |
| Weather            | AviationWeather.gov, Open-Meteo, RainViewer | Independent first-party provider plugins            | No account for the initial public endpoints               |
| AI ATC and crew    | SayIntentions.AI                            | Optional first-party account adapter                | Subscribed pilot account; SAPI is currently preview       |
| Online networks    | VATSIM and IVAO                             | Optional Atlas providers, later plugin capabilities | Public feeds need no user login; private APIs differ      |
| Simulator          | MSFS 2024 through SimConnect                | Primary WyrmGrid Bridge provider                    | Local simulator installation                              |
| Simulator          | MSFS 2020 and FSUIPC                        | Compatibility providers                             | Local installation; FSUIPC may be separately licensed     |
| Simulator          | X-Plane 12 Web API                          | Second simulator family                             | Local X-Plane 12.1.1 or later                             |
| Simulator audio    | MSFS 2024 and X-Plane 12                    | Separate capture and user-selected codec providers  | Separate microphone or communications approval            |
| Navigation         | Navigraph Navdata                           | Optional approved adapter                           | Developer approval and user subscription for current data |
| Airport baseline   | OurAirports                                 | Offline reference import                            | No account; public-domain dataset                         |
| Plan interchange   | MSFS, X-Plane, and Little Navmap formats    | First-party import/export adapters                  | No account                                                |
| Reminders          | OS notifications and iCalendar export       | Local core capability                               | No account                                                |
| Community delivery | Discord or other messaging                  | Deny-by-default plugin                              | User-selected service credentials                         |

## Delivery sequence

1. Complete fleet persistence and restart-time offline fallback.
2. Introduce the first canonical `FlightPlanSnapshot` with fixtures and
   provenance validation. (Implemented.)
3. Import a user's latest SimBrief OFP without storing a password or shared
   application secret. (Implemented as a Dispatch-session developer preview;
   a sanitized snapshot may also be retained with an explicitly created local
   flight recording. Authenticated live-field certification remains.)
4. Add cached airport weather and route-weather summaries. (Explicitly
   requested, session-cached METAR/TAF airport context, linked Atlas airport
   projection, and coarse along-route global-model context are implemented;
   route hazard advisories remain.)
5. Connect OnAir jobs, fleet selection, SimBrief plans, and explainable dispatch
   checks. (Registration, exact model-label, and current-airport comparison
   implemented; job payload and deadline checks await those OnAir contracts.)
6. Export simulator-neutral routes and implement the MSFS 2024 Bridge telemetry
   slice. (The read-only Bridge protocol, provider supervisor, SimConnect
   sidecar, external `.wyrmprovider` lifecycle, desktop controls, and plugin
   snapshot delivery are implemented; route export, live certification, and
   any licensed SimConnect-client redistribution remain.)
7. Add read-only SayIntentions local active-flight correlation through a reviewed
   documented transport and selected SAPI reads, followed by explicit
   user-initiated ACARS, crew, or gate actions.
8. Reconcile plan versus recorded time, track distance, altitude, fuel, airport
   proximity, and registration without inventing unavailable facts. (Version 1
   SimBrief/telemetry correlation implemented; payload, route adherence, phase
   detail, and AI ATC context remain.)
9. Add VATSIM and IVAO Atlas layers, followed by X-Plane and approved Navigraph
   features.
10. Add separately consented audio aligned to simulator sessions, beginning
    with capability-labelled Windows/MSFS sources and user-selected codec
    providers. (Capture protocol v2, codec protocol v1, consent, encrypted
    storage, deterministic fake capture, debug-only Windows microphone capture,
    and first-party Opus-as-a-provider are implemented; packaging and live
    certification remain.) Extend capture to Windows output and X-Plane only
    after their platform, legal, and plugin feasibility gates pass.

Provider work does not bypass the quality gates. Each protocol or schema change
needs sanitized fixtures, bounded validation tests, documentation, and an
explicit compatibility decision. Claims about authenticated or live behaviour
require a sanitized captured response or an outside-repository integration test.

The candidate register and implementation processes for the currently selected
SayIntentions, direct VATSIM, direct IVAO, and Navigraph work are documented in
the [high-value provider integration process](high-value-provider-process.md).

## Shared requirements

- Rust core adapters and explicitly approved provider-plugin SDK clients perform
  network and filesystem access; the Svelte interface remains presentational.
- Each provider plugin owns raw schemas, endpoint construction, and
  provider-specific translation. Core owns request selection, scheduling,
  caching, retry policy, validation, and safe error categories.
- The application exposes freshness and degraded state rather than substituting
  old data silently.
- Units are explicit at boundaries and normalized only in domain services.
- Provider identifiers and free-form text are treated as personal or untrusted
  until a narrower classification is proven.
- Sentry receives stable operation and error codes, never credentials, plan
  contents, routes, coordinates, callsigns, user IDs, or raw provider failures.
- Continuous audio never travels through Bridge's bounded JSON protocol.
  Microphones, communications, device labels, and external media remain behind
  a separate default-off capture and storage boundary.
- Plugins request specific capabilities. There is no `internet_access` or
  `provider_proxy` shortcut.
- External data is for simulation and planning assistance, not real-world
  operational use.

## Detailed plans

- [High-value provider integration process](high-value-provider-process.md)
- [SimBrief](simbrief.md)
- [SayIntentions.AI](sayintentions.md)
- [SayIntentions public-contract observation — 2026-07-19](sayintentions-contract-observation-2026-07-19.md)
- [Aviation weather](weather.md)
- [Weather radar](radar.md)
- [VATSIM and IVAO](online-networks.md)
- [Navigation data and flight-plan interchange](navigation-and-interchange.md)
- [WyrmGrid Bridge and simulator providers](wyrmgrid-bridge.md)
- [Simulator provider authoring and FSUIPC path](simulator-provider-authoring.md)
- [Simulator connection, recording, in-game control, and graphs](simulator-experience-roadmap.md)
- [Simulator-synchronised audio recording](simulator-audio-recording.md)
- [Audio Capture Provider protocol version 2](audio-capture-provider-protocol.md)
- [Audio provider authoring and packaging](audio-provider-authoring.md)
- [Audio Codec Provider protocol version 1](audio-codec-provider-protocol.md)
- [Telemetry lifecycle and SimBrief correlation](telemetry-plan-correlation.md)

## Local automation and community handoff

Job deadlines, lease events, maintenance windows, dispatch times, stale-data
warnings, and completed-flight summaries should support local operating-system
notifications and standards-based iCalendar export. Scheduling rules live in
Rust application services, survive restart through explicit persisted state, and
remain useful without a hosted account. The interface only displays and edits
the user's choices.

Discord webhooks, bots, email, and other community delivery mechanisms belong in
deny-by-default plugins or separately approved adapters. They receive a bounded,
user-selected notification payload and a specific delivery capability, not
OnAir credentials, provider tokens, arbitrary local files, or access to the full
database. Automatic public posting is off by default.

## Consciously deferred candidates

FlightAware, OpenSky, and similar real-world tracking feeds; unrestricted NOTAM
aggregation; openAIP; embedded third-party charts; and automatic posting to
community services are not part of the initial programme. They carry licensing,
privacy, cost, authority, duplication, or credential risks disproportionate to
their current value. They may be revisited through the same adapter and
capability boundaries when a concrete use case and permitted data source exist.

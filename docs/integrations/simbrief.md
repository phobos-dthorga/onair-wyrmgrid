# SimBrief integration plan

SimBrief is a first-party WyrmGrid integration. It connects OnAir dispatch inputs
to a detailed operational flight plan and later provides the baseline for
planned-versus-actual analysis. It is not an OnAir source of truth and is not a
substitute for WyrmGrid's own explainable compatibility checks.

## Verified public contract

As documented on 2026-07-14:

- a user's latest OFP can be fetched as XML by Pilot ID or username, with JSON
  available through the documented `json=1` option;
- generating a plan through the SimBrief API uses the user's SimBrief/Navigraph
  login and does not bypass SimBrief's AIRAC entitlement;
- developers must contact Navigraph for the supplied integration files and an
  API key;
- the documented generation flow is oriented around a browser popup and a PHP
  helper; and
- a caller-provided `static_id` can identify a generated plan independently of
  whichever plan the user later makes their newest OFP.

WyrmGrid must not claim more detailed field behaviour until a sanitized captured
response or an authenticated integration test outside the repository verifies
it.

## Implemented developer preview

The first read-only vertical slice is implemented behind WyrmGrid Dispatch:

- `wyrmgrid-simbrief-api` makes a user-requested HTTPS GET to the documented
  latest-OFP endpoint with `json=1`, follows no redirects, enforces a 15-second
  timeout and 2 MiB streaming limit, and returns only safe error categories;
- the Pilot ID or username exists only for the request and is never returned,
  persisted, logged, reported to Sentry, or exposed to plugins;
- raw JSON remains private to the adapter and is translated into canonical
  `FlightPlanSnapshot` version 1 groups with explicit mass units, timestamps,
  transformation version, freshness, and `external_calculation` provenance;
- the current Dispatch plan is session-only and can be cleared explicitly; and
- the interface renders origin, destination, alternates, aircraft, schedule,
  route, weights, fuel, AIRAC, and provenance only when supplied.

The repository fixture is synthetic and sanitized. It validates bounds,
translation, optional-field handling, provenance, and redaction, but it is not a
captured live response. Therefore the interface and documentation describe this
as a developer preview and do not claim certified live field compatibility. An
authenticated outside-repository capture remains a release gate.

## Phase 1: latest-OFP import (developer preview implemented)

- Let the user connect a SimBrief Pilot ID or username explicitly.
- Fetch JSON in a dedicated Rust adapter such as `wyrmgrid-simbrief-api`.
- Keep the raw response private and translate only verified fields into a
  `FlightPlanSnapshot`.
- Record generation time, retrieval time, provider revision or AIRAC, and an
  opaque plan correlation value where available.
- Detect that no plan exists, the identifier is invalid, or the latest plan has
  changed without treating those expected conditions as Sentry errors.
- Cache translated snapshots only after a user-visible persistence decision.
  The latest-OFP endpoint makes the identifier privacy-sensitive even though it
  is not an account password.
- Never ask for or store the user's SimBrief/Navigraph password.

Initial translation should cover only captured and tested fields for:

- origin, destination, alternates, and planned times;
- aircraft type and optional registration or airframe reference;
- route text and normalized waypoints when the payload supports them;
- payload, planned weights, fuel breakdown, endurance, and units;
- plan generation time and AIRAC; and
- links to OFP or simulator-format downloads only when their lifetime and use
  have been verified.

Unknown, malformed, or missing values remain absent. They are not replaced with
plausible defaults.

## Phase 2: WyrmGrid dispatch comparison

The first comparison increment is implemented for facts available in the
current OnAir fleet snapshot. It deterministically matches registration, can
identify a unique exact model-label candidate without claiming airframe
identity, compares the matched aircraft's current airport with the plan origin,
and retains both source values in every finding. Payload, aircraft limits, job
airports, schedules, and deadlines are visibly reported as unavailable until
those facts enter stable OnAir domain contracts; WyrmGrid does not infer them.

Application services compare, without silently merging:

- selected OnAir aircraft versus SimBrief aircraft type and airframe;
- job payload versus planned payload and weight constraints;
- OnAir origin and destination versus SimBrief airports;
- WyrmGrid schedule and job deadlines versus planned block times;
- WyrmGrid route recommendations versus SimBrief's route and alternates; and
- estimated range and fuel margins versus SimBrief fuel calculations.

Differences become explainable findings with both sources visible. SimBrief
calculations remain `external_calculation`; OnAir records remain `on_air_fact`.
If [SayIntentions.AI](sayintentions.md) has also imported a SimBrief plan,
WyrmGrid correlates only verified identifiers and presents any mismatch rather
than assuming both products are using the same OFP.

## Phase 3: generation and edit handoff

- Populate supported SimBrief dispatch inputs from an explicit user-selected
  WyrmGrid plan.
- Assign a collision-resistant, non-identifying `static_id` and retain it behind
  an opaque application reference.
- Open authentication and plan progress in the user's system browser unless
  Navigraph approves another secure native flow.
- Retrieve the resulting plan by its static correlation rather than assuming
  the newest plan is the one WyrmGrid generated.
- Require user confirmation before sending a plan to SimBrief. This is an
  external write even though the OnAir API remains read-only.

The shared SimBrief API key shown in the documented PHP flow must not be embedded
in a Tauri binary, JavaScript bundle, public repository, or GitHub Pages site. A
static GitHub Pages site cannot execute the server-side helper. Before this
phase, WyrmGrid must ask Navigraph whether a distributable desktop flow is
approved. If a shared key must remain server-side, add only a narrowly scoped
serverless broker after a separate hosting ADR, abuse analysis, retention
decision, and operating budget are accepted.

## Phase 4: simulator and actual-flight reconciliation

- Export the accepted route through the neutral flight-plan interchange layer.
- Load it into MSFS 2024 only after an explicit user action and a negotiated
  WyrmGrid Bridge capability.
- Compare planned and actual block/air times, route, altitude profile, fuel,
  diversion, and completion state.
- Store bounded operational summaries rather than unlimited raw telemetry.
- Never report an actual simulator result back to OnAir unless OnAir later
  documents and WyrmGrid separately approves a supported write operation.

## Security and privacy gates

- Treat Pilot IDs, usernames, plan IDs, OFP contents, routes, coordinates,
  callsigns, registrations, and download URLs as private operational data.
- Exclude all of them from Sentry and from plugins without a specific approved
  capability.
- Redact remote error bodies and URLs before they cross the adapter boundary.
- Validate download schemes, hosts, sizes, content types, and redirects.
- Store user tokens in the operating-system credential store if a future
  approved OAuth flow requires them.

## Required validation

- sanitized success, missing-plan, invalid-user, malformed-response, and schema
  evolution fixtures;
- mapping tests for units, timestamps, route order, optional fields, and AIRAC;
- a latest-plan replacement test and, later, a static-ID correlation test;
- redaction canaries in responses, errors, logs, and Sentry filters;
- timeout, offline, rate-limit, and cached-snapshot behaviour; and
- an outside-repository authenticated test before claiming generation or live
  download compatibility.

## References

- [SimBrief API introduction](https://developers.navigraph.com/docs/simbrief/introduction)
- [How the SimBrief API works](https://developers.navigraph.com/docs/simbrief/how-it-works)
- [Using the SimBrief API](https://developers.navigraph.com/docs/simbrief/using-the-api)
- [Fetching a user's latest OFP](https://developers.navigraph.com/docs/simbrief/fetching-ofp-data)
- [Navigraph developer terms](https://developers.navigraph.com/docs/developer-terms-of-service)

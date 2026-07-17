# Telemetry lifecycle and SimBrief correlation

## Shipped contract

WyrmGrid treats simulator telemetry and a SimBrief OFP as separate evidence
sources. A recording may retain a sanitized, validated `FlightPlanSnapshot`, but
does not turn that plan into simulator truth. Correlation schema version 2 is
owned by the Rust application service and can be revised independently of the
Bridge protocol, database migration number, and plugin API.

Automatic recording is optional and off by default. When enabled, WyrmGrid:

1. waits for two increasing, unpaused telemetry observations where SimConnect's
   direct `on_ground` fact is false;
2. begins one automatic session and retains both confirmation samples;
3. records the evidence as `takeoff_confirmed`;
4. after direct `on_ground` facts resume, waits for a continuous configurable
   settling period (30 seconds by default, bounded to 10–600 seconds); and
5. records `landing_settled` and completes only the automatically started
   session.

A pause, zero simulation rate, aircraft identity change, provider change, or
telemetry gap cannot silently complete lifecycle evidence. Gaps are stored as
events and reset landing settlement. Manual recordings never inherit automatic
stop authority.

## Plan association

After a successful SimBrief import, Dispatch supplies its validated domain
snapshot to the recording service. If a recording is active it is associated at
once; otherwise the next recording takes the current plan. Clearing Dispatch
prevents association with future recordings but deliberately does not rewrite a
recording that already captured the plan which was in force.

The database stores the sanitized domain snapshot, not the account reference
entered by the user, raw OFP JSON, HTTP response, URL, or credential. The plan
remains encrypted with the rest of WyrmGrid's SQLite data and is deleted with
its recording.

## Version 2 comparisons and overlays

The Hoard view labels plan and recording values separately. It may show:

- planned enroute time beside elapsed recorded time;
- planned route distance beside the sum of recorded coordinate segments;
- planned initial altitude beside peak recorded altitude;
- planned take-off fuel mass beside the first available recorded fuel weight;
- planned landing fuel mass beside the final available recorded fuel weight;
- the separately labelled non-negative recorded fuel delta;
- distance from the first/last recorded positions to sourced airport
  coordinates; and
- an exact, case-insensitive registration match when both sources contain a
  registration.

Version 2 also calculates signed recorded-minus-planned differences for
duration, distance, peak altitude, start fuel, and end fuel when both facts are
available. These pairs are observations, not performance scores. Initial altitude is not
called cruise-altitude compliance; track distance is not route adherence; a
fuel delta is not a dispatch fuel verdict; model labels are not treated as type
compatibility. Missing source fields remain unavailable.

The whole-flight debrief projects at most 250,000 retained source samples. Each
chart emits no more than 1,200 display samples. Recordings at or below that
display bound stay exact; longer traces use a deterministic min/max envelope
whose buckets select the extrema for every series on that chart. The first and
last samples are retained. If any omitted or selected source sample marks a gap,
the next represented point also marks that gap, so the renderer cannot connect
across missing telemetry. Fuel and position traces treat missing values as gaps
rather than inventing values. Requests above the source bound fail visibly
instead of allocating an unbounded interface payload.

SimBrief initial altitude and planned take-off/landing fuel appear as labelled
reference lines. Planned duration is a labelled time marker only when it falls
inside the recording. WyrmGrid does not invent a climb schedule, fuel-burn curve,
or route progress for a fix whose coordinates are absent. Planned and recorded
route progress is instead overlaid in Atlas as separate lines with the same gap
rules.

## Hoard ownership

Hoard exposes search across the bounded recording index, retention-resistant
pinning, exact 600-sample backward/forward windows, lifecycle events, and
user-initiated JSON or CSV export. JSON includes the session, exact samples,
events, correlation result, and associated sanitized plan; CSV is the exact
sample stream for analytical tools. Pinning protects a completed recording from
automatic retention pruning; it does not defeat explicit per-recording or
delete-all actions. Exports are user-controlled plaintext copies and therefore
leave SQLCipher protection.

The debrief, Atlas route handoff, and correlation result are host-owned
application views. They remain excluded from the plugin protocol. Exact
600-sample windows stay available as a separate forensic view and exports
continue to contain exact stored samples rather than downsampled chart points.

## Compatibility decision

Hoard Flight Debrief schema 1 and Atlas simulator-route schema 1 are additive
read models over migrations 8 and 10; they require no database migration and do
not alter export schema 1. Correlation version 2 re-evaluates an already stored,
sanitized plan snapshot and exact recording samples, so older associated
recordings gain the new comparisons without rewriting their historical facts.
Consumers must treat new optional comparison fields as unavailable when either
source fact is absent.

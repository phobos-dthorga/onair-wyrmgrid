# Telemetry lifecycle and SimBrief correlation

## Shipped contract

WyrmGrid treats simulator telemetry and a SimBrief OFP as separate evidence
sources. A recording may retain a sanitized, validated `FlightPlanSnapshot`, but
does not turn that plan into simulator truth. Correlation schema version 1 is
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

## Version 1 comparisons

The Hoard view labels plan and recording values separately. It may show:

- planned enroute time beside elapsed recorded time;
- planned route distance beside the sum of recorded coordinate segments;
- planned initial altitude beside peak recorded altitude;
- planned take-off fuel mass beside the non-negative recorded fuel delta;
- distance from the first/last recorded positions to sourced airport
  coordinates; and
- an exact, case-insensitive registration match when both sources contain a
  registration.

These pairs are observations, not performance scores. Initial altitude is not
called cruise-altitude compliance; track distance is not route adherence; a
fuel delta is not a dispatch fuel verdict; model labels are not treated as type
compatibility. Missing source fields remain unavailable.

Whole-session calculations are performed only while a session contains at most
50,000 exact samples. Above that bound WyrmGrid retains and pages through the
recording but withholds whole-session comparisons rather than presenting a
partial result as complete.

## Hoard ownership

Hoard exposes search across the bounded recording index, retention-resistant
pinning, exact 600-sample backward/forward windows, lifecycle events, and
user-initiated JSON or CSV export. JSON includes the session, exact samples,
events, correlation result, and associated sanitized plan; CSV is the exact
sample stream for analytical tools. Pinning protects a completed recording from
automatic retention pruning; it does not defeat explicit per-recording or
delete-all actions. Exports are user-controlled plaintext copies and therefore
leave SQLCipher protection.

Long-session whole-trace downsampling remains a separate follow-up. Until a
tested min/max-envelope contract ships, WyrmGrid describes each graph as an
exact selected window and does not imply that it depicts the entire flight.

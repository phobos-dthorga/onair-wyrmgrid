# First vertical slice

The first product milestone proves this complete path:

```text
credential entry
  -> session-only Rust credential holder
  -> OnAir company GET connection probe
  -> optional operating-system credential storage (later opt-in milestone)
  -> sequential OnAir fleet and FBO GET requests
  -> raw response validation
  -> stable domain translation
  -> timestamped SQLite snapshot
  -> application state
  -> Atlas aircraft and FBO layers
  -> selection inspector
  -> background refresh without UI blocking
```

## Acceptance criteria

- No secret appears in logs, errors, plugin messages, crash reports, or SQLite.
- A connection can be verified, identified by company name, and disconnected
  without persisting the API key.
- A company, its fleet, and its FBO network can be viewed after a successful refresh.
- The last successful snapshot remains usable while offline.
- Every displayed record shows data age and source.
- Failed refreshes preserve the last valid state and explain the failure.
- Fixture-backed tests cover translation without requiring contributor secrets.

Write operations, worldwide market enumeration, weather, simulator telemetry,
route optimization, and the public plugin runtime are outside this slice.

The session-only connection probe is the first completed increment of this
slice. It intentionally precedes persistent credential storage and fleet
synchronization so each security boundary can be tested in isolation.

The next completed increment translates the official fleet envelope into stable
aircraft and airport summaries and presents valid locations through the Atlas
Fleet layer and linked inspector. Hoard now persists successful observations,
restores the newest compatible company fleet at startup, labels offline and
cached states explicitly, and retains hourly then daily history. The next
increment adds the narrow Swagger-verified FBO identity and airport mapping to
the same pipeline, with independent persistence and partial-failure handling.

Those retained observations now power the first Hoard Timeline slice.
Historical queries resolve fleet and FBO resources as-of a selected time and
preserve each resource's real observation timestamp rather than invent an
atomic company snapshot. The workspace keeps historical mode separate from
live/cached/offline availability and returns to LIVE explicitly or at restart.
Fleet size, ranked fleet composition, and FBO-network size are calculated in
the application service from those same bounded retained observations.

The first operational-provider increment adds canonical `FlightPlanSnapshot`
version 1 and a session-only, read-only SimBrief latest-OFP developer preview in
Dispatch. The next increment compares observed OnAir registration, model, and
aircraft position without collapsing either source, reports unavailable payload
and deadline evidence rather than guessing, and adds an explicitly requested
ten-minute session cache of AviationWeather.gov METAR and TAF facts for plan
airports. Route advisories, online-network, navigation, simulator, and persistent
operational caches remain sequenced in the
[external integrations programme](../integrations/README.md).

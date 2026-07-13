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

Those retained observations deliberately form the basis of the future Hoard
Timeline. Historical queries will resolve resources as-of a selected time and
preserve each resource's real observation timestamp rather than invent an
atomic company snapshot.

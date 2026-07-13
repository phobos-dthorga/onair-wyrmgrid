# First vertical slice

The first product milestone proves this complete path:

```text
credential entry
  -> session-only Rust credential holder
  -> OnAir company GET connection probe
  -> optional operating-system credential storage (later opt-in milestone)
  -> OnAir fleet GET requests
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
- A company and its fleet can be viewed after a successful refresh.
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
aircraft and airport summaries, retains the latest observation in process
memory, and presents valid locations through the Atlas Fleet layer and linked
inspector. SQLite snapshot persistence, restart-time offline fallback, and FBO
translation remain before this vertical slice is complete.

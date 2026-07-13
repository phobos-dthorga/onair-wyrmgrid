# WyrmGrid Hoard

Hoard is WyrmGrid's local SQLite history and offline-data boundary. It stores
stable WyrmGrid observations after raw OnAir responses have been validated and
translated. Raw OnAir JSON and API credentials never enter Hoard.

## Fleet and FBO snapshots

After every successful resource synchronization, the application stores a
schema-versioned payload containing:

- the company identifier, name, and airline code;
- stable aircraft and current-airport summaries;
- stable FBO identity and airport summaries;
- the original OnAir provenance source and observation time;
- the WyrmGrid snapshot schema version.

The existing migration-1 `api_snapshots` table already owns this generic record
shape, so the fleet and FBO persistence slices do not change the database schema
or add an unnecessary migration. Future schema changes must use new append-only
migrations and must not edit migration 1.

At desktop startup, the newest compatible fleet snapshot across known companies
is loaded before an OnAir connection exists. Atlas presents it immediately as
**Offline · Hoard snapshot**. Connecting a company switches to its newest saved
snapshot, marked **Cached**, while a live synchronization is pending. Only a
successful remote observation becomes **Live**.

The matching company FBO snapshot is restored independently. Fleet and FBO
observations retain their own timestamps because the OnAir API does not expose
an atomic combined response. A successful resource remains usable even when the
other resource fails during the same guarded synchronization.

Failed synchronization never replaces the last valid observation. It marks the
retained data Cached, keeps the map usable, and reports the bounded network or
API failure separately.

## Retention

Resource history is compacted transactionally whenever a successful observation is
saved:

- retain the newest observation in each UTC hour for the most recent seven days;
- retain the newest observation in each UTC day for older history;
- partition retention by company and resource kind.

This provides useful recent resolution and long-term daily history without
allowing ordinary synchronization to produce unbounded intraday growth.

## Hoard Timeline

Hoard is more than an offline fallback. Its observation history supports a
read-only **Hoard Timeline** where a player can inspect how a charter operation
or airline changed over time.

The timeline uses an **as-of** model: for each requested resource, WyrmGrid
selects the newest compatible observation at or before the chosen UTC time.
Fleet, finance, FBO, route, job, and flight observations may have different
capture times because the public OnAir API does not provide one atomic company
snapshot. The interface must show each resource's actual observation time and
must not imply that independently collected facts were simultaneous.

The application always exposes one of two mutually exclusive workspace
modes: **LIVE** or **HISTORICAL**. Mode is separate from data availability. For
example, the current operational workspace may honestly read **LIVE mode ·
Offline snapshot**, while a past view reads **HISTORICAL mode · As of 12 March
2026**. This avoids hiding loss of connectivity and prevents a saved observation
from masquerading as a current server response.

HISTORICAL mode:

- display a persistent mode indicator, **Viewing history** banner, and selected
  time;
- remain read-only and visually distinct from Live, Cached, and Offline data
  availability states;
- keep live synchronization running independently without moving the user's
  selected historical position;
- provide an explicit return-to-present action;
- derive growth and change metrics from stable observations rather than store
  recommendations as facts.

The implemented first timeline slice reuses retained fleet and FBO observations
and provides a globally visible LIVE/HISTORICAL mode, time selector,
return-to-present control, fleet-growth chart, and selected fleet-composition
chart. FBO-network growth is calculated from the independently timestamped FBO
snapshot history and presented alongside fleet growth without implying that the
two resources were observed simultaneously. Dense fleet compositions use a
ranked horizontal presentation so full real-world fleets remain readable. The
Rust application service owns as-of resolution and the fleet/FBO growth and
composition calculations; Tauri commands and Svelte remain presentation
adapters. History reads are bounded to the newest 4,096 compatible observations
per company and resource.

Historical selection is intentionally not persisted across application
restarts. WyrmGrid always starts in LIVE mode so an old point in time cannot be
mistaken for the present after reopening the application. Live synchronization
continues while the user inspects history and does not move the selected point.

Later slices can add company value, geographic FBO coverage, route network,
utilization, finance, and user-named milestones as those stable domain resources
become available. Charts will consume the same query results as tables and Atlas
so all three presentations share one definition of the underlying history.

The existing generic snapshot records, company/resource partitioning, schema
versions, observation timestamps, and hourly-to-daily retention policy remain
the foundation. A separate history store or event-sourcing system was not
required for the initial timeline, so no database migration was added.

## Failure mode

The desktop opens `wyrmgrid.db` in the operating system's application-data
directory. If that location cannot be created or SQLite cannot be opened,
WyrmGrid degrades to an in-memory store rather than preventing startup. Atlas
then labels successful observations **Memory only**, making the loss of restart
persistence visible.

Unsupported or malformed stored payload versions are ignored safely. Hoard does
not reinterpret them as current domain data.

## Privacy boundary

The database contains company identifiers, company names, fleet records,
locations, and observation history. It does not currently encrypt those facts
at rest and relies on the user's operating-system account and file permissions.
Users should treat `wyrmgrid.db` as private operational data and sanitize it
before attaching it to an issue. API keys are never written to this database.

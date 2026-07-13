# WyrmGrid Atlas

Atlas is WyrmGrid's shared geographical workspace. It is expected to evolve for
the lifetime of the project, so it grows through narrow vertical slices rather
than one grand map implementation.

## Data boundary

```text
OnAir response
  -> private raw Rust types
  -> stable domain aircraft
  -> timestamped application snapshot
  -> thin Tauri command
  -> declarative Atlas layer
  -> linked selection inspector
```

MapLibre and Svelte receive application-owned summaries only. They do not parse
raw OnAir JSON, infer business state, hold credentials, or decide what a remote
status code means.

## First fleet slice

The first Atlas slice provides:

- an authenticated, user-triggered fleet refresh;
- stable aircraft and airport summaries translated in Rust;
- OnAir provenance and observation time for the complete fleet snapshot;
- aircraft markers for records with valid WGS84 coordinates;
- current-airport coordinates as a fallback when direct aircraft coordinates
  are absent;
- a Fleet layer toggle and separate received/mapped counts;
- automatic map fitting after a new fleet observation;
- marker selection linked to an aircraft inspector;
- preservation of the last in-memory observation when a later refresh fails;
- a clearly labelled synthetic browser-preview fleet for interface testing.

The committed fleet fixture and browser-preview data are synthetic. They contain
no user company, aircraft, airport, or credential data.

## Deliberate limits

This slice does not yet provide SQLite persistence, restart-time offline data,
FBOs, clustering, routes, jobs, range rings, maintenance, or plugin-published
layers. Those should be added only when the preceding layer establishes the
smallest shared contract they require.

Atlas layers should remain declarative. A future plugin may publish bounded
features and presentation metadata, but it must not receive the MapLibre object
or execute arbitrary map code in the desktop webview.

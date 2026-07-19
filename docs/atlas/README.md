# WyrmGrid Atlas

Atlas is WyrmGrid's shared geographical workspace. It is expected to evolve for
the lifetime of the project, so it grows through narrow vertical slices rather
than one grand map implementation.

## Data boundary

```text
OnAir response
  -> private raw Rust types
  -> stable domain aircraft or FBO
  -> timestamped, schema-versioned snapshot
  -> SQLite Hoard history
  -> live/cached/offline application view
  -> thin Tauri command
  -> declarative Atlas layer
  -> linked selection inspector
```

MapLibre, Three.js, and Svelte receive application-owned summaries only. They
do not parse raw OnAir JSON, infer business state, hold credentials, or decide
what a remote status code means.

## First fleet and FBO slices

The first Atlas slice provides:

- an authenticated initial, manual, or automatic company-data synchronization;
- a clearly labelled manual synchronization control that remains available
  during its silently enforced quiet period;
- locally remembered automatic-check preferences with conservative interval
  choices;
- locally remembered Atlas layer visibility, plus an optional last-view restore
  that stores bounded camera values only while the user enables it;
- stable aircraft and airport summaries translated in Rust;
- OnAir provenance and observation time for the complete fleet snapshot;
- aircraft markers for records with valid WGS84 coordinates;
- current-airport coordinates as a fallback when direct aircraft coordinates
  are absent;
- a Fleet layer toggle and separate received/mapped counts;
- automatic map fitting after a new fleet observation;
- marker selection linked to an aircraft inspector;
- a separately toggleable gold FBO layer with airport-backed locations;
- FBO selection linked to an inspector without coupling it to aircraft state;
- independent fleet and FBO snapshots so a partial remote failure does not
  discard the resource that succeeded;
- persistent, schema-versioned Hoard snapshots after successful observations;
- immediate restart-time display of the latest cached company fleet;
- explicit Live, Cached, Offline, Preview, Hoard, and Memory-only labels;
- preservation of the last valid observation when a later refresh fails;
- a visible LIVE/HISTORICAL workspace mode and Hoard Timeline that can project
  independently captured fleet and FBO facts into Atlas as of a retained time;
- fleet, composition, and FBO-network growth charts derived from the same
  retained facts shown by Atlas;
- return-to-present behavior that preserves live observations while history is
  being inspected;
- clearly labelled synthetic browser-preview fleet and FBO data for interface testing.

The current Dispatch route slice additionally provides:

- a Rust-owned, projection-versioned Atlas route view derived from the current
  session-only validated flight plan;
- stable plan-scoped selection IDs for origin, destination, alternates, and
  ordered route fixes, including duplicate identifiers;
- coordinate-only route points and dashed segments which break at unresolved
  fixes rather than inventing geometry;
- antimeridian-safe full-route framing including mapped alternates; and
- linked Dispatch actions for the full route, airports, weather stations, and
  route fixes, with an explicit Atlas inspector result when a coordinate is
  unavailable.

The committed fixtures and browser-preview data are synthetic. They contain
no user company, aircraft, airport, or credential data.

## Dispatch plan explorer

The first linked route slice projects the current validated SimBrief snapshot
from Dispatch into Atlas. Origin, destination, alternates, and route fixes use
stable host-issued selection IDs. Full-route framing and focused navigation use
only provider-supplied coordinates; missing locations remain selectable and
visible in the inspector but are not plotted or bridged by a line. Alternates
remain separate markers rather than becoming invented route legs.

Historical simulator debriefs reuse the same planned-route projection beside a
separate bounded recorded trace. This keeps provenance and missing-evidence
rules identical between the live plan and Hoard history.

## Regional lens

Atlas bundles a versioned, locally served ADM1 snapshot covering sourced
states, provinces, regions, territories, prefectures, and equivalent first-level
divisions. Hover raises a region through feature-state styling without moving
its actual border; click or tap pins its source and identifiers in the shared
inspector. Reduced-motion and low-resource runs retain the facts with a quieter
effect.

The data, terminology, licensing, disputed-boundary posture, reproducible build
path, and zoom-gated ADM2 county/district design are defined in
[Administrative regions in Atlas](administrative-regions.md).

## Deliberate limits

This slice does not yet provide FBO capacity, fuel, workshop, pricing, or
construction details, nor route procedures, range rings, or maintenance.
Detailed source-shaped weather is implemented through a failure-safe
MapLibre/Three.js composition; its present limits and future WebGPU path are
recorded in [Atlas Three.js weather renderer](weather-renderer.md). The UTC
daylight, truthful weather-support-zone, RADAR-footprint, and future eclipse
boundaries are recorded in
[Atlas daylight, weather coverage, and eclipse plan](daylight-weather-coverage-and-eclipses.md).
Atlas also renders the host-selected recent RADAR frame and its explicit
no-coverage mask, with timestamped motion-safe playback controls, and overlays
Rust-derived global-model context along continuous Dispatch route segments.
Remaining features should be added only when the
preceding layer establishes the smallest shared contract they require.

Atlas layers should remain declarative. A future plugin may publish bounded
features and presentation metadata, but it must not receive the MapLibre object
or execute arbitrary map code in the desktop webview.

Automatic scheduling currently belongs to the desktop while authoritative
serialization and quiet periods belong to the Rust application service. This
keeps one small timer near the active window lifecycle without duplicating API
protection policy in Svelte.

Hoard persistence, as-of historical resolution, fleet-history calculations, and
the live/cached/offline decision belong to the Rust application and storage
services. Svelte receives explicit live and historical views and only chooses
how those states are presented.

The route-selection, clickable SID/STAR, live/historical weather, Hoard, and
GPU-profile contract is detailed in
[Flight plans and weather in Atlas](flight-plan-and-weather.md).

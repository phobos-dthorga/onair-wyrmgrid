# Flight plans and weather in Atlas

This document defines how Dispatch, SimBrief, weather, Hoard, and Atlas should
join without creating a second interpretation of the same operational facts.
It is the design contract for staged increments. Atlas now draws the current
coordinate-only Dispatch plan, its current airport-weather observations, and a
bounded planned-versus-recorded route for a selected historical simulator
recording. It does not yet resolve navigation geometry. Atlas now animates
bounded source-shaped weather graphics without interpolating sparse reports,
plays a bounded recent RADAR timeline, and projects coarse global-model context
along continuous plan segments.

## One plan, two projections

Dispatch owns the current session's validated `FlightPlanSnapshot`. Atlas is
the spatial projection of that same application view. Atlas must not fetch or
reparse a SimBrief OFP, retain a second mutable route, or infer missing legs in
Svelte.

```text
private SimBrief response
  -> validated FlightPlanSnapshot
  -> Dispatch textual projection
  -> Atlas route projection
  -> shared selection and provenance
```

The implemented projection is produced in the Rust application service and is
included with `DispatchStatus`. It assigns plan-scoped stable IDs to the origin,
destination, alternates, and every ordered route fix. Each feature explicitly
reports either `resolved` with a source coordinate or
`location_unavailable`; duplicate identifiers remain distinct through their
sequence-based IDs.

Atlas draws only supplied coordinates. A missing route fix breaks the rendered
line rather than joining across the unknown location, while Dispatch keeps the
unresolved item clickable and Atlas explains the gap in its inspector. Full
route framing includes every mapped route feature and alternate and uses
antimeridian-safe bounds. The browser preview uses clearly synthetic
coordinate-bearing plan data solely for interface validation.

The route layer provides:

- origin, destination, and alternate airport markers;
- an ordered geodesic route line through every resolved leg;
- distinct airport, alternate, route-fix, and unresolved-location
  presentation, while direct and procedure classifications remain gated on a
  future stable navigation contract;
- a **View full route** action that fits origin, resolved legs, destination,
  and relevant alternates, including antimeridian-safe bounds; and
- a route inspector that retains SimBrief provenance, generation/retrieval
  times, AIRAC, source route text, and any resolution warnings.

The current snapshot has ordered identifiers, optional airways, and optional
coordinates. It does not yet establish SID/STAR membership or procedure
geometry. WyrmGrid must therefore not guess that a token is a STAR from its
spelling or position. Procedure interaction becomes available after the
navigation resolution service supplies a stable leg ID, procedure kind,
coordinates, and source/AIRAC provenance.

## Linked interaction

Dispatch and Atlas should exchange stable selections rather than map objects:

- selecting an origin, destination, alternate, route fix, SID, or STAR in
  Dispatch opens Atlas and focuses the matching resolved feature;
- selecting a STAR focuses its complete geometry and arrival airport, then
  opens the shared inspector rather than jumping to an unrelated map copy;
- selecting an airport-weather card focuses that airport and activates the
  relevant weather layer/time; and
- **View full route** clears a narrow selection and frames the complete run
  from departure to destination.

An unresolved item remains clickable in Dispatch but produces an honest
"location unavailable" result and can still show its identifier and
provenance. It must never be silently dropped or plotted at a plausible
location. AIRAC disagreement between the plan and navigation source remains
visible and may prevent procedure geometry from being joined.

### Current Dispatch plan explorer

`DispatchStatus.atlas_plan` is an additive application view using
`FlightPlanMapView` schema 1. Rust constructs it from the already validated
snapshot; Dispatch and Atlas never independently parse or resolve route text.
Every origin, destination, alternate, and route leg receives a stable point ID
for the lifetime of that plan. Dispatch sends only that ID when the user opens
the complete route or a particular point.

Atlas plots only points with supplied valid coordinates. Missing points remain
in the inspector as **Location unavailable** and break the line before the next
resolved point. Alternates are selectable markers but are not joined to the
filed route. Full-route framing includes all sourced plan locations and handles
the antimeridian. A point selection focuses its sourced position; selecting an
unresolved point opens the same plan and provenance without inventing a camera
target.

The inspector shows point kind, airway where supplied, provider, retrieval
time, and whether the point belongs to the filed route. The complete plan view
also reports resolved and unresolved counts. Current plan state remains
session-only and clearing Dispatch removes its source snapshot; the Atlas
projection contains no independent persistent copy.

### Historical recording route slice

Hoard Flight Debrief route-view schema 2 is the first shared-projection route
layer. It
contains only coordinates already present in the recording's sanitized
`FlightPlanSnapshot` and its bounded recorded position trace. Plan and recording
are separate MapLibre line features. A gap splits its source line; a missing plan
coordinate remains a selectable `FlightPlanMapPoint` and also splits the plan.
Planned fix markers retain their identifiers, and an antimeridian-safe fit
frames both sources together. Dispatch and Hoard now reuse the same host-built
`FlightPlanMapView`, so missing-coordinate and provenance rules cannot drift.
Svelte does not resolve or infer geometry. The route remains local and is not
published to community plugins or sent to the public basemap service as
feature data.

`FlightPlanMapView` schema 1 changes only the internal application/read-model
contract. It adds `atlas_plan` to Dispatch status. Debrief route-view schema 2
replaces its schema-1 planned-route fields with the same projection; route
views are regenerated from retained source facts rather than persisted, so no
database migration or legacy reader is required before public release. This
does not change the plugin protocol, simulator sidecar protocol, SQLite schema,
or canonical `FlightPlanSnapshot` version.

The selection contract belongs in Rust application/domain types. Svelte may
request `focus route`, `focus airport`, or `focus feature`, while Atlas alone
owns camera animation and fit bounds.

## Live weather projection

Atlas weather layers consume translated immutable weather products, not raw
provider JSON, images, or arbitrary remote map styles. Each feature carries at
least source, product kind, issue/observation time, validity interval, retrieval
time, freshness, coverage, and geometry provenance.

### Current airport-weather projection

`DispatchStatus.atlas_weather` is an additive `FlightWeatherMapView` schema-1
projection built in Rust by joining the validated plan airports with the
current `WeatherSnapshot`. It gives origin, destination, and alternates stable
selection IDs and carries only source coordinates and translated METAR/TAF
observations. Dispatch can open the complete weather layer or one station in
Atlas; Atlas displays category, wind, visibility, raw reports, timestamps, and
provenance in its inspector.

A station with coordinates but no report is plotted as **Unknown**, never
clear. A report whose plan airport lacks coordinates remains part of the
evidence view and inspector contract but is not plotted. The weather view is
session-only, disappears when its source plan is cleared, and does not change
the canonical plan, weather snapshot, plugin protocol, simulator protocol, or
database schema.

The intended layers are incremental:

1. airport condition symbols and wind/visibility context for the plan's
   airports;
2. supported SIGMET, AIRMET, G-AIRMET, and related advisory geometries;
3. sourced wind and temperature fields with level and valid-time controls;
4. licensed or otherwise approved radar and satellite imagery; and
5. optional high-detail animation and volumetric presentation where the source
   data legitimately supports it.

### Current along-route model projection

`DispatchStatus.route_weather` is now an additive schema-2 application view.
Rust samples continuous mapped plan segments about every 300 nautical miles,
derives a schedule basis from the validated plan, and assigns a proportional
ETA to each checkpoint. Scheduled-off precedes scheduled-out; a positive
estimated enroute duration precedes a positive scheduled-on or scheduled-in
window. Missing or invalid timing remains explicit rather than being replaced
with the local clock.

The bundled Open-Meteo layer carries six UTC forecast horizons for each of the
fixed 84 global locations. Rust selects a source only when it lies within both
the 1,200-nautical-mile spatial limit and the three-hour temporal limit. The
view retains checkpoint ETA, source valid time, signed time offset, point
identity, distances, provider, retrieval time, freshness, and supplied numeric
fields. Deterministic ties resolve by absolute time offset, spatial distance,
then point ID. An older plugin point without a valid time is still accepted as
**current context**, but never called a forecast.

Missing plan coordinates split the corridor. Missing schedule data, forecast
horizon, or distant model support stays unavailable or current-only as
appropriate. Svelte formats this view and builds declarative Atlas line
features; it does not select sources, calculate ETAs, interpolate conditions,
or decide route suitability. Solid coloured sections are ETA-matched, dashed
coloured sections are current context, and neutral dashed sections are
unsupported.

The same view includes the newest retained RADAR frame metadata for each
provider layer. It is labelled observation-only. Atlas may open the factual
RADAR timeline and its no-data masks, but neither Rust nor Svelte projects cell
movement or substitutes a past frame for future weather. The display remains
broad simulation-planning context, not an aviation briefing or a claim that
weather between coarse samples is known.

Schema 2 is an internal regenerated application projection. Adding optional
`valid_at` to global grid points is backward-compatible in plugin API version
1: old responses deserialize unchanged, the `forecast_grid` request remains
identical, and no database migration is required.

Absence of a report is not rendered as clear weather. Sparse station
observations must not be interpolated into a photorealistic atmospheric claim.
A high-end GPU permits richer rendering of sourced data; it does not create
additional weather knowledge.

### Source-shaped phenomena

High-detail effects are welcome when an adopted source supports them:

- a radar or approved precipitation field may drive rain/snow intensity,
  movement, accumulation colour, and GPU particle density;
- timestamped lightning events or cells may drive flashes, illumination, and
  strike visualisation within the supplied spatial and time precision;
- provider-coded convective, icing, turbulence, cloud, and visibility products
  may drive bounded volumes or surfaces when their geometry and levels are
  preserved; and
- wind fields may drive particles or streamlines at the selected valid time and
  altitude.

The visual must not imply finer evidence than the product contains. For
example, a METAR reporting a thunderstorm can support a sourced storm symbol or
airport-area effect, but not invented strike coordinates. Exact-looking bolts
require a lightning source; precipitation motion requires either observed time
steps or a clearly labelled forecast/advection model. WyrmGrid should render
uncertainty through softened boundaries, age/freshness, gaps, and product
resolution rather than filling missing space with plausible spectacle.

Enhanced and cinematic profiles may make the same supported phenomenon more
beautiful through volumetric light, particles, reflections, shadows, and
temporal smoothing. They must not add unsupported phenomena, shift their
location, extend their valid time, or hide the source and observation controls.

## Historical weather and Hoard

Historical playback requires a new persisted weather-snapshot contract. The
current Dispatch weather cache is process-memory-only and is not historical
evidence.

When implemented, Hoard should retain bounded translated snapshots and expose
them through the same time model used by Atlas history:

- **Live** uses the newest valid product and continues normal refresh policy;
- **Historical** freezes the selected Hoard time and resolves the newest
  observation known at or before it, without making network requests to rewrite
  the past;
- observation time, issue time, forecast valid time, retrieval time, and Atlas
  playback time remain separate visible concepts; and
- gaps, expired products, changed coverage, and missing imagery remain visible.

Historical radar/satellite retention needs an explicit source-licence,
storage-volume, deletion, and offline-use decision before caching begins. Hoard
must not retain unlimited image tiles or raw provider payloads by default.

## Rendering profiles

Display size is not a performance proxy. A handheld or low-resolution laptop
may have a capable GPU, while a large remote-desktop window may not. Atlas uses
an explicit rendering preference plus capability probing.

| Profile       | Default | Intended presentation                                                                                      |
| ------------- | ------- | ---------------------------------------------------------------------------------------------------------- |
| Compatibility | No      | Airport symbols and labels with conservative texture, animation, and memory budgets                        |
| Enhanced      | Yes     | GPU heatmap atmosphere, sourced wind vectors, and gentle condition motion around reporting airports        |
| Cinematic     | No      | Layered cloud depth, dense precipitation marks, convective illumination, and dust volumes for capable GPUs |

The user-facing **Enhanced** preference is enabled by default. Users may choose
Compatibility or Cinematic and may independently disable cloud,
precipitation, lightning, and dust graphics. The `--low-resource` launch switch
always forces Compatibility for that run; it does not rewrite the persisted
preference. Reduced Motion keeps every detailed profile static.

The station treatment remains deliberately station-shaped. Enhanced and
Cinematic lazily initialize a separately composited Three.js
`WebGPURenderer`, preferring WebGPU and accepting its WebGL2 fallback. MapLibre
retains markers, labels, wind vectors, radar, and a complete decorative fallback
if Three.js cannot initialize or loses its device. WebGPU ray-marches a bounded
shared procedural density texture for local cloud, obscuration, and dust
volumes; WebGL2 uses layered mesh and point substitutes. It uses only
structured wind and explicit METAR present-weather/category fields, labels the
display source-shaped, and does not interpolate between airports. Rain and snow
marks, dust layers, cloud shading, and the convective lightning symbol remain
anchored to their station report. Grid effects use only validated provider
points, while RADAR uses the selected frame from a bounded six-frame provider
timeline. A separate coverage mask makes no-data pixels visibly grey. The
current lightning graphic is a storm-cell symbol and local illumination, not a
strike. Satellite imagery, forecast animation, persisted RADAR history, exact
lightning events, measured GPU-time/VRAM budgets, and
persisted device-loss telemetry remain future source and renderer work. A local
submission-pressure controller can temporarily reduce visual ceilings without
changing the selected profile or factual layers. The active Three.js backend
and device-loss fallback are reported in the Atlas presentation.
Atlas also compares the source coordinate with MapLibre's projection round trip
and fades decorative cells whose anchor falls behind the globe or pitched-map
horizon. Ray-marched volumes use a deterministic screen-stable sample offset
to reduce slicing at bounded step counts. Neither mechanism supplies terrain
depth, a shared GPU render graph, or new meteorological evidence.

Every profile must preserve facts, timestamps, selected time, hazards, and
accessibility. Lower profiles reduce rendering cost, never data correctness.
Reduced-motion, battery, remote-session, and thermal considerations remain
independent overrides. If an Enhanced feature misses its frame-time, memory, or
device-loss budget, Atlas degrades the individual effect and reports the active
profile instead of losing the route or weather layer.

**Reduce flashes** is a separate safety preference and remains enabled by
default in every rendering profile. Enhanced or cinematic selection does not
turn it off. Disabling it requires a plain-language photosensitivity warning
and explicit confirmation; WyrmGrid must not infer consent from accepting the
Terms, choosing a capable GPU profile, or having used the setting previously on
another device. The first-run Terms summary discloses that future weather and
warning effects may flash, while the runtime control prevents or reduces them.

## Security and privacy

- Pilot IDs, OFP content, routes, coordinates, registrations, and historical
  movement are private operational data.
- Atlas receives only stable application views; MapLibre, Three.js, community
  plugins, diagnostics, and Sentry do not receive raw plans or weather
  payloads.
- Community layers cannot access the map object, replace WyrmGrid route/weather
  layers, or counterfeit provenance and freshness labels.
- A future plugin capability for plan or historical-weather access requires a
  separate authorization decision; current fleet, map-publication, and live
  simulator grants do not imply it.
- Remote tiles, imagery, and styles require an approved host allowlist, content
  bounds, attribution, cache policy, and threat-model update.

## Delivery sequence and validation

1. Add a stable Atlas route-view model and full-route framing using only
   coordinates already present in the snapshot. **Implemented.**
2. Add shared Dispatch/Atlas selection IDs and explicit unresolved-leg results.
   **Implemented.**
3. Introduce navigation resolution with procedure/AIRAC provenance before
   clickable SID/STAR geometry.
4. Project current airport weather, followed by validated advisory geometries.
   The airport projection and linked inspector are implemented; route hazards
   remain later work.
5. Add append-only bounded weather persistence and Hoard historical playback.
6. Add Compatibility/Enhanced settings and a truthful first GPU airport-weather
   treatment. **Implemented.**
7. Add Cinematic and phenomenon-specific controls with visible precipitation,
   source-shaped clouds, convective illumination, dust graphics, flash safety,
   and low-resource/Reduced Motion overrides. **Implemented.** Measured
   frame-time/VRAM budgets and persisted device-loss telemetry remain future
   work.
8. Add a lazy Three.js `WebGPURenderer`, automatic WebGL2 and MapLibre
   fallbacks, bounded render-scene projection, initial hard resource ceilings,
   and device-loss reporting. **Implemented.** See the
   [renderer contract](weather-renderer.md).
9. Add bounded TSL ray-marched WebGPU cloud/obscuration/dust volumes, a shared
   deterministic 3D density field, WebGL2 visual substitutes, and conservative
   adaptive pressure levels. **Implemented.** Measured GPU timing, VRAM
   accounting, and cross-WebView visual baselines remain future work.
10. Add projection-round-trip horizon fading and deterministic screen-stable
    ray-sample stratification without private MapLibre renderer access or
    frame-varying noise. **Implemented.** Terrain/building depth and volume-wide
    globe intersection still require a supported shared rendering contract.

Tests must cover route order, unresolved legs, duplicate identifiers, procedure
classification, AIRAC mismatch, full-route bounds, antimeridian crossing,
weather validity and gaps, historical as-of resolution, profile persistence,
low-resource override, device loss, hostile geometry, and identical factual
content across rendering profiles.

Open decisions include the approved navigation geometry source, whether a
future MapLibre WebGPU backend exposes a supported shared-device/depth custom
layer contract, weather imagery licensing, historical retention limits, and
the measurable frame-time/VRAM thresholds for each profile.

Radar source evaluation, immutable frame requirements, rendering profiles, and
historical gates are defined in the
[weather radar integration contract](../integrations/radar.md).

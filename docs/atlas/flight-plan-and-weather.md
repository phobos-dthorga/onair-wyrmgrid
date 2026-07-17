# Flight plans and weather in Atlas

This document defines how Dispatch, SimBrief, weather, Hoard, and Atlas should
join without creating a second interpretation of the same operational facts.
It is the design contract for the current coordinate-only flight-plan
projection and future weather increments. Atlas does not yet resolve navigation
geometry or draw animated weather.

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

The first route layer should provide:

- origin, destination, and alternate airport markers;
- an ordered geodesic route line through every resolved leg;
- distinct direct, airway, procedure, and unresolved-leg presentation only
  after those classifications enter the stable navigation contract;
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

The selection contract belongs in Rust application/domain types. Svelte may
request `focus route`, `focus airport`, or `focus feature`, while Atlas alone
owns camera animation and fit bounds.

## Live weather projection

Atlas weather layers consume translated immutable weather products, not raw
provider JSON, images, or arbitrary remote map styles. Each feature carries at
least source, product kind, issue/observation time, validity interval, retrieval
time, freshness, coverage, and geometry provenance.

The intended layers are incremental:

1. airport condition symbols and wind/visibility context for the plan's
   airports;
2. supported SIGMET, AIRMET, G-AIRMET, and related advisory geometries;
3. sourced wind and temperature fields with level and valid-time controls;
4. licensed or otherwise approved radar and satellite imagery; and
5. optional high-detail animation and volumetric presentation where the source
   data legitimately supports it.

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

| Profile                | Default | Intended presentation                                                                                                        |
| ---------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Compatibility          | Yes     | Airport symbols, bounded vector advisory shapes, static contours, reduced animation, conservative texture and memory budgets |
| Enhanced               | No      | GPU-instanced particles, animated wind fields, smoother time interpolation, higher-resolution approved imagery               |
| Experimental cinematic | No      | Sourced volumetric or three-dimensional effects behind explicit feature flags and measured budgets                           |

The user-facing preference should initially read **Prefer compatibility weather
rendering** and be enabled by default. A user can opt into Enhanced rendering
after WyrmGrid reports the detected graphics API and estimated cost. The
`--low-resource` launch switch always forces Compatibility for that run; it does
not rewrite the persisted preference.

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
- Atlas receives only stable application views; MapLibre, community plugins,
  diagnostics, and Sentry do not receive raw plans or weather payloads.
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
5. Add append-only bounded weather persistence and Hoard historical playback.
6. Add Compatibility/Enhanced settings, GPU capability reporting, budgets, and
   device-loss fallback before high-detail effects.

Tests must cover route order, unresolved legs, duplicate identifiers, procedure
classification, AIRAC mismatch, full-route bounds, antimeridian crossing,
weather validity and gaps, historical as-of resolution, profile persistence,
low-resource override, device loss, hostile geometry, and identical factual
content across rendering profiles.

Open decisions include the approved navigation geometry source, MapLibre custom
layer versus a separately composited renderer, WebGL2/WebGPU support policy,
weather imagery licensing, historical retention limits, and the measurable
frame-time/VRAM thresholds for each profile.

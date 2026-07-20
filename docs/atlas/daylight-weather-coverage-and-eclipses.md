# Atlas daylight, weather coverage, and eclipse plan

Status: UTC daylight and twilight, conservative weather support zones, and
validated RADAR tile footprints are approved for the current Atlas slice.
Eclipse rendering remains a designed future source-backed feature.

## Product intent

Atlas should answer three different questions without blending their evidence:

1. **Where is sunlight reaching Earth now?** This is an astronomical
   calculation from UTC and position.
2. **Where does the current weather product provide support?** This comes from
   airport observation anchors, complete regular forecast grids, or received
   raster footprints.
3. **Where is a weather phenomenon actually present?** This requires a source
   that supplies suitable spatial geometry or imagery. A point observation is
   not such a boundary.

The visual hierarchy keeps routes, aircraft, FBOs, place labels, and selection
markers readable. Daylight shading sits above the basemap but below operational
symbols. Weather support zones remain beneath their point markers and Three.js
volumes. The separate layer toggles allow a user to remove either treatment
without changing data or graphics-quality preferences.

## Daylight and twilight

Live Atlas calculates the solar position from the device's current UTC time.
Historical Atlas uses the selected Hoard time, so the displayed daylight does
not silently return to the present while company facts are frozen in the past.
The calculation follows the NOAA fractional-year, equation-of-time, solar
declination, hour-angle, and solar-zenith equations described in
[General Solar Position Calculations](https://gml.noaa.gov/grad/solcalc/solareqns.PDF).
It does not require a provider, network request, location permission, or local
time-zone database.

Atlas renders four non-overlapping spherical bands around the anti-solar point:

| Band                  | Geometric solar elevation | Colour             | Purpose                                            |
| --------------------- | ------------------------: | ------------------ | -------------------------------------------------- |
| Civil twilight        |                 0° to -6° | blue-grey, 12%     | transition immediately after sunset/before sunrise |
| Nautical twilight     |               -6° to -12° | slate blue, 21%    | stronger low-light context                         |
| Astronomical twilight |              -12° to -18° | deep navy, 31%     | very low natural illumination                      |
| Night                 |                below -18° | midnight blue, 43% | clearly unlit portion of the globe                 |

A restrained amber terminator line marks geometric sunrise and sunset. The
bands use spherical destination calculations and small bounded polygons rather
than a flat-map gradient, so they follow the globe, antimeridian, polar day,
and polar night. They refresh once per minute. This is sufficient because the
terminator moves roughly a quarter degree of longitude per minute and avoids a
per-frame CPU or GPU cost.

The overlay is a geometric illumination guide. Terrain, buildings, cloud
shadow, atmospheric refraction, local horizon obstruction, and artificial
lighting are outside its claim. NOAA notes that apparent sunrise/sunset
calculations add atmospheric-refraction assumptions; WyrmGrid deliberately
uses the geometric 0° boundary for a stable global layer.

## Weather support zones

The layer is named **Weather support zones**, not storm extent. Its colour and
pattern semantics remain stable across themes:

| Condition            | Zone colour       | Repeating pattern               |
| -------------------- | ----------------- | ------------------------------- |
| Cloud/overcast       | cool grey-blue    | gently layered horizontal bands |
| Rain                 | medium cyan-blue  | slanted rain strokes            |
| Snow                 | pale ice blue     | sparse flake crosses            |
| Convective/lightning | muted coral-red   | sharp zigzag/chevrons           |
| Obscuration          | muted violet-grey | staggered stipple               |
| Dust                 | warm ochre-brown  | diagonal crosshatch             |
| RADAR tile           | restrained cyan   | square scan grid                |

Colour is always paired with pattern, shape, a bounded outline, and the
original weather marker; it is not the only carrier of meaning. The patterns
are generated locally as small power-of-two tiles so they remain seamless and
crisp without bundled artwork or a network fetch. Reduced-resource mode keeps
the same categorical patterns at lower opacity rather than removing the
non-colour distinction.

### Airport observations and Three.js cells

METAR and airport weather remain point observations. Atlas may draw two soft,
semi-transparent rings and the matching patterned disc behind a non-clear
local weather effect so the user can associate the marker, detailed volume,
and nearby map context. The rings are an **indicative observation vicinity**
with no nautical-mile or storm-boundary claim. Their soft edge and fine outline
intentionally distinguish them from sourced polygons. They must never be used
for route clearance, avoidance, or a claim that weather stops at the ring.

### Model samples and source-reported extents

When a validated plugin layer contains every point in a complete rectilinear
latitude/longitude grid, Atlas calculates a compact circular support footprint
centred on each source sample. Its radius is no larger than half the nearest
sample spacing and is capped at 180 nautical miles. Coarse grids therefore
leave visible unknown gaps instead of allowing one reading to colour a
continent-sized region. Each circle is a **model sampling support area**. It
identifies its nearest validated sample; it does not state that a weather front
follows the circle or that an unfilled gap is clear weather.

Irregular, incomplete, duplicate, or single-row grids do not receive inferred
support circles. Their validated points remain visible without fabricated
coverage. A point may instead carry an explicit provider-reported extent radius
when its first-party product actually defines the geographic extent of that
weather pattern. Atlas then uses the validated radius rather than grid spacing,
and identifies its basis as provider-reported. It never derives pattern size
from precipitation intensity, cloud cover, icon category, or neighbouring
samples. An extent that crosses the antimeridian remains absent until Atlas can
split that geometry safely.

The bundled AviationWeather.gov and Open-Meteo products do not currently supply
such a weather-pattern extent, so their size does not grow or shrink. A future
provider with sourced cell polygons, uncertainty, altitude, valid time, and
resolution can supersede the circular presentation after a separate contract
and compatibility review.

### RADAR

The current RainViewer integration supplies bounded raster tiles rather than
individual RADAR station locations or ranges. Atlas can therefore outline the
exact geographic footprint of each validated received tile and retain the
existing raster as the precipitation evidence. The outline does not mean every
pixel has a measurement and does not create a RADAR-station range. The bundled
provider now supplies RainViewer's separate coverage PNG with each tile;
transparent pixels are covered and black pixels are unavailable. Atlas renders
unavailable pixels in neutral grey and retains the tile outline as the received
product footprint. A missing mask still does not make transparency mean clear
weather.

Station rings become possible only after the host-owned RADAR contract carries
a validated station identifier, location, scan radius/beam geometry, product
time, resolution, and station geometry. The evidence and licensing gates
remain those in the [RADAR integration contract](../integrations/radar.md).

## Eclipse extension

An eclipse is not implemented by moving or darkening the ordinary night cap.
An accurate solar-eclipse display needs the Moon's penumbral, umbral, or
antumbral shadow relative to the rotating Earth, together with contact times
and uncertainty. NASA GSFC describes the standard Besselian-element method and
publishes the fundamental-plane coordinates, shadow radii, declination, and
hour-angle inputs used to calculate those paths in
[Besselian Elements of Solar Eclipses](https://eclipse.gsfc.nasa.gov/SEcat5/beselm.html).

The future implementation should:

1. adopt a versioned, attributed catalogue of eclipse events and Besselian
   elements rather than a loose browser calculation;
2. translate it into a stable Rust domain/application projection containing
   event identity, source revision, valid interval, contacts, partial-coverage
   polygon, central path, umbral/antumbral limits, and uncertainty;
3. validate time bounds, finite coefficients, polygon size, winding,
   antimeridian handling, and source attribution before Svelte receives it;
4. evaluate the event at Live or selected historical Atlas time and render the
   partial penumbra separately from the much narrower total/annular path;
5. use a restrained violet penumbra, a high-contrast gold path edge, a labelled
   centre line, and optional hatching so the display remains understandable
   without colour;
6. keep normal twilight visible: partial eclipse reduces direct sunlight but
   does not turn daytime into astronomical night;
7. disclose prediction uncertainty and never present a visual edge as a
   guarantee of local viewing conditions; and
8. add catalogue fixtures, known-contact/path tests, historical-time tests,
   polar and antimeridian cases, attribution, documentation, and a compatibility
   decision before enabling the layer.

NASA's prediction material notes that path edges can still carry kilometre-
scale uncertainty from the lunar limb profile and that long-range historical
or future times depend on Earth-rotation uncertainty. That is why eclipse
geometry is planned rather than approximated in this slice.

## Ownership and validation

- `atlas/daylight.ts` owns solar calculations and bounded spherical shade
  geometry.
- `weather/weatherCoverage.ts` owns regular-grid support cells and received
  RADAR tile footprints.
- `AtlasMap.svelte` owns MapLibre sources, layer ordering, visibility, and
  theme-independent presentation only.
- The root workspace owns the session layer toggles and the Live/Historical
  time supplied to Atlas.

Tests cover equinox and solstice declination, subsolar and anti-solar elevation,
twilight geometry bounds, deterministic feature counts, complete-grid
midpoints, incomplete-grid refusal, antimeridian-safe cells, and exact RADAR
tile footprints. Browser verification must inspect globe rotation, polar views,
label readability, Three.js composition, reduced-resource mode, and production
build output.

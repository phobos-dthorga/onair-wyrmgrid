# Weather radar integration

Status: bounded RainViewer current-frame preview implemented through an
independent provider plugin; animation, history, and higher-resolution products
remain deferred.

Radar is a time-varying spatial weather product, not a larger METAR symbol.
Atlas will eventually place validated radar frames beneath route, airport, and
hazard overlays while keeping the source time, coverage, gaps, and resolution
visible. A powerful GPU can make sourced echoes fluid and beautiful; it cannot
turn missing coverage into precipitation or determine what weather a simulator
has selected.

## Evidence boundary

WyrmGrid keeps these products separate:

1. **External real-world weather** — attributed METAR, TAF, radar, satellite,
   lightning, advisory, and forecast products.
2. **Simulator weather configuration** — a simulator-reported mode such as
   live, preset/custom/scenario, or unknown, only when the simulator contract
   proves it.
3. **Simulator ambient observations** — bounded conditions reported at the
   aircraft, such as precipitation state, visibility, pressure, temperature,
   wind, or cloud density.

Similarity between an external radar frame and simulator ambient conditions is
not proof that live weather is active. WyrmGrid must not infer the simulator
mode from visual or numeric resemblance. Historical analysis may compare the
streams, but it labels them independently and preserves their own clocks.

## Host-owned radar contract

An adopted radar adapter translates private provider payloads into a bounded,
immutable application product. Every frame must carry:

- provider, product and provider revision;
- observation/scan time, valid time where applicable, and retrieval time;
- geographic projection, extent, resolution, and coverage description;
- a provider-defined or validated value legend and units;
- an explicit missing-data/no-coverage mask distinct from a measured zero;
- frame sequence and animation interval where multiple observations exist;
- freshness, attribution, and licence/caching constraints; and
- checksums or equivalent stable identity for cache integrity.

The provider plugin owns URL construction and raw-response translation. Rust
owns product validation, bounds, scheduling, caching, request correlation, and
as-of selection. Svelte and MapLibre receive only a host-built render projection. Remote styles,
scripts, arbitrary tile URLs, and provider credentials never enter the browser
contract.

## Rendering and time

The Compatibility profile uses static or deliberately low-rate imagery with a
small texture/memory budget. Enhanced rendering may use GPU colour mapping,
temporal interpolation between actual adjacent frames, reprojection, terrain
masking, particles driven by supported fields, and higher-resolution textures.
Cinematic rendering is an explicit local preference. It increases the detail
of the same validated products and never expands radar coverage or precision.

All profiles show the same factual frames. Interpolation is presentation
between two observations, not a new observation, and stops across missing
frames or incompatible products. No-data areas remain transparent or visibly
hatched rather than being interpreted as clear skies.

**Reduce flashes** remains enabled by default. Radar animation itself should
avoid abrupt full-screen luminance changes. Lightning illumination or warning
flashes require the separate safety preference and explicit confirmation
defined in the Atlas weather contract.

Live mode selects the newest valid frame under the provider policy. Historical
mode resolves only frames retained in Hoard at or before the selected time and
never fetches current data to rewrite the past. Retention begins only after
licensing, storage, deletion, offline-use, and backup implications are approved.

## Provider assessment

No source is approved merely because it can be displayed in a browser.

| Candidate                        | Useful evidence                                                | Current decision                                                                                                                                                    |
| -------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AviationWeather.gov Data API     | Worldwide aviation reports and advisories                      | Current public product list does not establish a radar imagery feed; do not fabricate radar from station reports.                                                   |
| NOAA/NCEI NEXRAD                 | High-quality near-real-time and historical U.S. radar evidence | Strong U.S. candidate, but not global; delivery, redistribution, attribution, volume, and derived-product details need a dedicated adapter review.                  |
| Australian Bureau of Meteorology | Australian radar imagery and supported data services           | Anonymous-feed and commercial/republication conditions vary by product; adopt only a specifically licensed service, not scraped public imagery.                     |
| MetService New Zealand           | Some radar imagery is described through its data-access policy | Exact product/API access, reuse, cache, attribution, and commercial terms need written confirmation before implementation.                                          |
| MSFS MapView weather radar       | Documented in-simulator presentation capability                | It does not establish a raw, redistributable desktop radar feed and cannot stand in for external observations.                                                      |
| RainViewer                       | Global composite tile timeline                                 | Adopted for the initial simulation-only preview: latest past frame, four host-selected zoom-one PNG tiles, five-minute refresh, no remote tile URLs in the webview. |

Official starting points:

- [RainViewer Weather Maps API](https://www.rainviewer.com/api/weather-maps-api.html)
- [AviationWeather.gov Data API](https://aviationweather.gov/data/api/)
- [NOAA/NCEI NEXRAD](https://www.ncei.noaa.gov/products/radar/next-generation-weather-radar)
- [Bureau of Meteorology data feeds](https://www.bom.gov.au/catalogue/data-feeds.shtml)
- [Bureau of Meteorology data services](https://www.bom.gov.au/resources/data-services)
- [MetService data-access policy](https://about.metservice.com/our-data-access-policy)
- [MSFS 2024 MapView weather radar mode](https://docs.flightsimulator.com/msfs2024/html/6_Programming_APIs/WASM/MapView_API/fsMapViewSetWeatherRadarMode.htm)

## Delivery gates

1. Approve one product with documented access, coverage, licence, attribution,
   redistribution, offline, and historical-retention terms.
2. Capture sanitized metadata/geometry fixtures and test malformed, oversized,
   missing, stale, mixed-projection, and antimeridian cases.
3. Implement one static current-frame application view with a visible legend,
   time, coverage, and no-data treatment.
4. Add bounded animation and renderer fallbacks with identical factual content
   across profiles.
5. Add append-only Hoard persistence only after retention size, pruning,
   backup, export, and deletion policies are approved.
6. Add lightning, volumetric precipitation, or forecast motion only from an
   adopted product that explicitly supports that phenomenon and precision.

Provider adoption requires a threat-model update and local performance tests;
it does not require or authorize an application release, version bump, tag, or
hosted CI run.

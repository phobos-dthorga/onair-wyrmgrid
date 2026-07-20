# Aviation weather integration plan

Weather uses stable, provider-neutral core contracts with three independent
first-party provider plugins: AviationWeather.gov airport reports, Open-Meteo
global model samples, and RainViewer radar tiles.

WyrmGrid weather is for flight-simulation planning. It must not be presented as
a real-world briefing or as evidence that a flight is operationally safe.

## Implemented airport-weather slice

The first read-only increment is implemented behind WyrmGrid Dispatch:

- the user explicitly requests weather after importing a plan;
- the AviationWeather.gov plugin sends one bounded JSON request each to the
  documented METAR and TAF endpoints for at most ten normalized plan-airport
  identifiers;
- the adapter follows no redirects, uses a custom WyrmGrid user agent, enforces
  a 15-second timeout and 512 KiB streaming limit per product, returns safe error
  categories, and keeps raw JSON private;
- the application coalesces concurrent refreshes, applies a one-minute retry
  floor, reuses a successful combined result for ten minutes, and retains the
  last valid result if a later refresh fails;
- immutable `WeatherSnapshot` version 1 products retain source, generated,
  retrieved, validity, transformation, and freshness metadata; and
- Dispatch shows raw coded METAR and TAF text plus a small allowlisted set of
  provider-decoded METAR fields without converting them into a go/no-go score.
- Rust builds `FlightWeatherMapView` schema 1 from those same snapshots and the
  validated plan, allowing Dispatch to open the complete weather layer or one
  airport directly in Atlas without reparsing provider data in Svelte.

The sanitized fixtures follow the official field contracts and public
responses captured on 2026-07-14. No provider credentials or private operational
identifiers are present. The cache is currently process-memory only; persistent
offline weather and route hazard products remain future increments.

The current Dispatch status also includes a Rust-built, time-aware along-route
model view. The bundled Open-Meteo plugin requests hourly UTC values for the
same fixed 84 global locations and publishes six bounded horizons: the first
available hour and hours 3, 6, 9, 12, and 18. The 504 resulting points remain
under the existing 512-point and one-megabyte product limits. Every forecast
point carries its own `valid_at`; provenance retrieval time is not relabelled as
a model-run time.

The host correlates each timed sample to the original fixed location through a
strict `<host-point-id>-hNN` identifier and exact coordinate match. Every
requested location must remain represented, and every returned sample must map
to exactly one request point. This preserves the original one-point version-one
contract while allowing the bundled six-horizon product without exposing the
route or widening the existing product limits.

Rust samples each continuous, coordinate-bearing plan segment at an interval no
greater than 300 nautical miles. It derives departure from scheduled-off then
scheduled-out and derives duration from estimated enroute time, scheduled-on,
or scheduled-in. Checkpoint ETAs are proportional to mapped route distance. A
forecast is accepted only within 1,200 nautical miles and three hours of the
checkpoint ETA. Older plugin points without `valid_at` remain compatible but
are visibly labelled **current context**, never an ETA forecast. Missing plan
timing, forecast horizon, or spatial support stays explicit.

The fixed global request means the plan and schedule never cross into the
plugin or Open-Meteo. Atlas draws only the horizon nearest retrieval time for
ordinary global weather graphics, avoiding six overlapping volumes per grid
location, while the complete temporal product remains available to the Rust
route analysis. Atlas uses a dashed corridor for current-only context and a
neutral dashed line for unavailable sections. Dispatch shows checkpoint ETAs,
forecast valid times, time offsets, source distances, and the latest factual
RADAR timestamp. RADAR remains current/past observation context and is never
extrapolated to a route ETA or converted into a safety finding.

## Initial products

| Product                         | Initial use                                                             |
| ------------------------------- | ----------------------------------------------------------------------- |
| METAR/SPECI                     | Current departure, destination, alternate, and fleet-airport conditions |
| TAF                             | Conditions across the planned departure and arrival window              |
| SIGMET and international SIGMET | Route and destination hazard overlays                                   |
| AIRMET and G-AIRMET             | Regional hazard context with coverage clearly labelled                  |
| PIREP/AIREP                     | Optional observed hazard context, never guaranteed coverage             |
| Winds and temperatures          | Later route and fuel context after field validation                     |

Products with regional coverage must say so. Absence of an advisory or report
does not prove safe or clear conditions.

## Adapter and data model

- Make requests from the approved out-of-process provider through the bounded
  Python SDK client. The Svelte interface must not call providers directly.
- Keep raw JSON, GeoJSON, XML, or text private to its provider plugin.
- Translate into immutable observations and advisories with source, issue time,
  observation time, validity interval, retrieval time, location or geometry,
  raw-versus-decoded status, and freshness.
- Preserve raw coded text only where it is useful and safe to display; never
  make an unverified local decoder appear authoritative.
- Associate weather with a plan by airport or bounded route corridor in an
  application service. Do not embed an entire OFP or user route in cache keys,
  logs, or telemetry.
- Store the last valid translated snapshot for offline use and visibly mark it
  stale when its product-specific validity has passed.
- Keep SimBrief- or SayIntentions-bundled weather distinct from
  AviationWeather.gov observations; show the source and time rather than
  silently replacing one with the other.

## Request policy

The official service asks clients to use a custom user agent, limits requests to
100 per minute, says an endpoint should not be consumed more often than once per
minute per thread, limits most responses to 400 entries, and recommends cache
files for large datasets.

WyrmGrid therefore starts more conservatively:

- coalesce identical station and route requests in the Rust application layer;
- cache METAR requests for at least five minutes and TAF requests for at least
  ten minutes unless official product metadata requires a longer interval;
- refresh route advisories no more frequently than every five minutes while a
  relevant workspace is open;
- apply bounded exponential backoff to `429` and transient server errors;
- use published bulk cache files instead of issuing large station-by-station
  request bursts; and
- stop automatic refresh while offline or when no weather-dependent view or
  active operation needs it.

These are WyrmGrid operating limits, not claims about provider guarantees.

## User-facing capabilities

1. Airport weather cards with source, age, raw text, and cautious decoding.
2. Departure, destination, and alternate comparison at relevant plan times.
3. Atlas layers for supported advisory geometries.
4. ETA-matched along-route model context with visible spatial and temporal
   support; future hazard findings must remain explainable and must not become
   a hidden score.
5. Cached offline viewing with prominent expired or stale status.

Weather must not be reduced to a single hidden score. Recommendations retain the
observations, thresholds, and uncertainty that produced them.

## Real weather and simulator weather

External weather and simulator weather are not interchangeable. WyrmGrid uses
three separately attributed evidence streams:

- external real-world observations, forecasts, radar, and advisories;
- the simulator-selected weather mode or scenario, when an SDK contract reports
  it; and
- ambient conditions observed by telemetry around the simulated aircraft.

The mode may eventually be recorded as `live`, `preset_or_custom`, or `unknown`
only from verified simulator evidence. It must not be inferred by comparing
ambient values with an external report. Mode changes and ambient samples may be
stored with a recording after a versioned Bridge protocol change, fixtures, and
compatibility decision; external weather remains a separate source snapshot
with its own observation and retrieval times.

This allows a debrief to say, for example, that the simulator was using a
custom scenario while external airport weather reported something else. It
does not label the player's chosen weather as false, overwrite it with external
data, or imply that simulator ambient observations were real-world conditions.

GPU-enhanced rain, snow, cloud, wind, and lightning are valid presentation
goals only when the adopted product supplies the corresponding phenomenon,
spatial precision, and valid time. A coded thunderstorm observation may support
a storm indication but not fabricated strike locations. The detailed
source-shaped rendering rule lives in the
[Atlas flight-plan and weather contract](../atlas/flight-plan-and-weather.md#source-shaped-phenomena).

## Required validation

- sanitized fixtures for each adopted product and each response format used;
- `204`, malformed data, partial fields, unknown codes, and provider-error tests;
- unit, timestamp, validity-window, geometry, and antimeridian tests;
- request coalescing, cache expiry, backoff, and maximum-result tests;
- stale-data and conflicting-provider presentation tests; and
- threat-model coverage for hostile text, oversized geometry, and decompression
  limits if bulk gzip files are adopted.

WIFS, hazard geometry, persisted RADAR history, higher-resolution model grids,
and commercial weather sources remain deferred. Each introduces authentication,
licensing, rendering, volume, or coverage questions beyond this initial slice.

The specific radar source and rendering gates are defined in the
[weather radar integration contract](radar.md).

The planned Atlas projection, historical Hoard model, conservative-by-default
rendering profile, and optional GPU-enhanced presentation are defined in the
[Atlas flight-plan and weather contract](../atlas/flight-plan-and-weather.md).

## References

- [AviationWeather.gov Data API](https://aviationweather.gov/data/api/)
- [AviationWeather.gov WIFS](https://aviationweather.gov/wifs/)
- [Open-Meteo Forecast API](https://open-meteo.com/en/docs)
- [Open-Meteo licence and attribution](https://open-meteo.com/en/license)
- [RainViewer Weather Maps API](https://www.rainviewer.com/api/weather-maps-api.html)

# Aviation weather integration plan

Weather is a first-party external-facts adapter. The initial source is the
public AviationWeather.gov Data API because it provides a documented,
machine-readable boundary for worldwide METAR and TAF data plus several route
hazard products.

WyrmGrid weather is for flight-simulation planning. It must not be presented as
a real-world briefing or as evidence that a flight is operationally safe.

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

- Make requests from Rust. AviationWeather.gov currently does not permit CORS,
  so the Svelte interface must not call it directly.
- Keep raw JSON, GeoJSON, XML, or text private to a cohesive weather adapter.
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
4. Route-weather findings explaining which sourced condition affected a score.
5. Cached offline viewing with prominent expired or stale status.

Weather must not be reduced to a single hidden score. Recommendations retain the
observations, thresholds, and uncertainty that produced them.

## Required validation

- sanitized fixtures for each adopted product and each response format used;
- `204`, malformed data, partial fields, unknown codes, and provider-error tests;
- unit, timestamp, validity-window, geometry, and antimeridian tests;
- request coalescing, cache expiry, backoff, and maximum-result tests;
- stale-data and conflicting-provider presentation tests; and
- threat-model coverage for hostile text, oversized geometry, and decompression
  limits if bulk gzip files are adopted.

WIFS, graphical forecast imagery, radar tiles, and commercial weather sources
are deferred. Each introduces authentication, licensing, rendering, volume, or
coverage questions beyond the initial airport and route-weather slice.

## References

- [AviationWeather.gov Data API](https://aviationweather.gov/data/api/)
- [AviationWeather.gov WIFS](https://aviationweather.gov/wifs/)

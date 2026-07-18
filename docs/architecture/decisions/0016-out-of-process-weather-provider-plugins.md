# ADR-0016: Out-of-process weather provider plugins

Status: accepted

## Context

WyrmGrid already separates provider payloads from stable domain models, but the
first AviationWeather.gov slice is compiled into the application. Global model
weather and radar would otherwise add more provider-specific clients, parsing,
refresh rules, and failure modes to the core. That is a poor fit for a small
FOSS project that should remain useful when any optional service is absent.

The plugin developer preview already supplies process isolation, explicit
capability review, bounded framed messages, and host-owned Atlas rendering. It
does not yet have a weather request/response contract or enforceable network
sandbox.

## Decision

Open-Meteo, AviationWeather.gov, and RainViewer are shipped as three independent
first-party Python plugins. Each owns only its provider URL construction and
raw-response translation. Shared behaviour remains in WyrmGrid core:

- immutable weather observations, forecast grids, and raster-tile products;
- product validation, provenance, bounds, freshness, and safe error categories;
- request scheduling, station and grid selection, cache replacement, and
  last-valid-result behaviour;
- capability approval and process supervision; and
- MapLibre rendering, accessibility, resource limits, and fallback behaviour.

Plugin protocol version 1 gains additive `weather_request` and
`publish_weather` messages, a `weather_data_publish` capability, declared
weather-product capabilities, and declared HTTPS origins. Existing version-one
plugins neither request nor receive weather messages and remain compatible.
Changing an existing message meaning, removing a field, or making a new field
mandatory for an old plugin still requires a new protocol or plugin API
version.

Declared origins make user review and authorization scope changes explicit;
they do not create an operating-system network sandbox. The bundled providers
use the SDK's bounded HTTPS client, reject redirects, cap response sizes, and
send only translated products over stdout. Unreviewed community plugins remain
unsafe to trust merely because their manifest declares an origin.

The initial products are deliberately bounded:

- AviationWeather.gov returns airport `WeatherSnapshot` facts for an explicit
  set of at most ten normalized ICAO stations.
- Open-Meteo returns a coarse host-selected global forecast grid. The plugin
  does not decide the sampling density or invent observations between samples.
- RainViewer returns a small, current set of validated PNG radar tiles. Remote
  scripts, styles, tile templates, and credentials never enter the webview.

All three sources are external real-world or model evidence for simulation use.
They do not establish the simulator's selected weather mode and are never
presented as a real-world operational briefing.

## Consequences

Provider failure stops or degrades only that provider. Atlas can render model
weather, airport facts, and radar independently with visible provenance and
age. The application can add future providers without adding raw payload types
to Svelte or duplicating rendering rules.

The first-party plugins require Python 3 and explicit user approval. Process
separation is not a sandbox, raster frames are intentionally low-resolution,
and direct provider subscriptions or API keys are deferred until a host-owned
credential and destination broker can keep secrets out of plugin messages and
the webview.

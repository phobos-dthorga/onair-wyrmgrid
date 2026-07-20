# Near-future ActiveSky weather plugin case

**Status:** documented candidate; no implementation or support claim

ActiveSky should remain an optional WyrmGrid weather provider. It should not
become a dependency of SimBrief import, historical-weather reconstruction, or
ordinary Dispatch and Atlas use.

## Why it belongs in a plugin

ActiveSky represents the weather selected for a simulator session. That is a
different evidence stream from AviationWeather.gov observations, Open-Meteo
model history, RainViewer radar, and ambient conditions read back from the
simulator. Keeping it out of core preserves all of these distinctions:

- WyrmGrid remains useful when ActiveSky is absent or stopped;
- a simulator weather engine cannot silently replace external historical or
  current sources;
- provider availability, version, time, and spatial precision remain visible;
- access to any local API, exported file, or process remains deny-by-default;
  and
- a provider failure degrades one layer rather than Dispatch, Atlas, SimBrief,
  recording, or the simulator connection.

SimBrief can use an ActiveSky snapshot supplied through its own supported user
workflow. That does not prove that an imported OFP contains ActiveSky's full
weather field, exact cells, or later simulator state. WyrmGrid may show
sanitized SimBrief source metadata as a provenance hint only after it appears
in a captured, reviewed contract. It must never infer an ActiveSky connection
from the age or values of the imported weather.

## Proposed product behavior

When a verified ActiveSky interface is available, the provider would run out
of process and publish a provider-neutral weather product through Forge. Atlas
would label it **Simulator weather — ActiveSky**, with the source time and the
simulator/scenario relationship made explicit.

The first useful product should be a bounded host-selected grid or documented
provider geometry. It should not receive the SimBrief OFP, pilot identity,
OnAir state, flight history, simulator credentials, or an unrestricted map
object. If an approved local interface needs a route, that is a later,
separately authorized capability with a privacy review.

ActiveSky may publish a circular pattern extent only when the verified
first-party contract supplies a defensible circular radius. If it supplies
polygons, rasters, cells, or another geometry, WyrmGrid should preserve that
shape instead. Point values, precipitation intensity, cloud cover, pressure,
or visual similarity must never be converted into an invented storm radius.

## Transport and permission boundary

No local ActiveSky SDK or transport contract has yet been accepted into the
repository. Before implementation, the maintainer must capture and sanitize
the applicable official interface for the supported ActiveSky and simulator
versions, including authentication, licensing, rate, redistribution, and
failure behavior.

The present plugin network helper permits only approved exact HTTPS origins.
Do not weaken it to allow arbitrary loopback, LAN, filesystem, registry, or
process access. A verified ActiveSky transport should use one of these narrow
designs:

1. an exact loopback origin with a new host-mediated loopback capability,
   fixed port/transport rules, no redirects, and explicit consent; or
2. a separately supervised local provider sidecar that translates the
   official interface and emits the existing framed, bounded WyrmGrid weather
   contract.

Either choice is a security and protocol decision. It requires fixtures,
compatibility rules, threat-model changes, and renewed permission. The plugin
must not receive or persist an ActiveSky account credential unless a later
credential design explicitly authorizes it.

## Selection and precedence

WyrmGrid should never silently combine conflicting sources. A future source
selector may offer:

- **External current weather** — current observations, model forecasts, and
  radar with their own providers;
- **Historical reconstruction** — historical METAR observations and model
  history for an imported past plan; and
- **Simulator weather — ActiveSky** — the weather intended for or reported by
  the simulator session.

If more than one is visible, each remains a separately attributed layer.
ActiveSky does not become the historical fallback merely because SimBrief used
an uploaded ActiveSky snapshot. Historical use requires a verified ActiveSky
archive with an exact requested time and the same explicit historical labels.

## Delivery gates

1. Capture a sanitized official ActiveSky contract for each supported product
   and simulator combination.
2. Decide the exact transport, licence, supported versions, and absence-safe
   behavior.
3. Threat-model local endpoint discovery, spoofing, replay, oversized data,
   credentials, and cross-user/process access.
4. Add a versioned capability, manifest scope, fixtures, validation, and an
   explicit compatibility decision.
5. Implement the provider translator and bounded session cache outside the UI.
6. Add success, unavailable, stopped, stale, malformed, wrong-version,
   conflicting-source, and source-extent tests.
7. Make the UI opt-in, source-explicit, and incapable of silently overriding
   external current or historical layers.

Until those gates are complete, ActiveSky is a documented integration case,
not a WyrmGrid-supported provider.

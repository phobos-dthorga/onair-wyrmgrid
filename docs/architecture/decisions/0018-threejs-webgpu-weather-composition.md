# ADR-0018: Three.js WebGPU-preferred weather composition

Status: accepted

## Context

Atlas already uses MapLibre GL JS for its globe, terrain, vector and raster
layers, labels, selection, and map interaction. MapLibre's current web renderer
uses WebGL2. Its custom layers can share that WebGL context and depth buffer,
but cannot host Three.js's WebGPU backend. MapLibre is developing a WebGPU
backend, but no current supported contract guarantees that an application can
share its GPU device, command stream, or depth texture.

WyrmGrid needs richer source-shaped clouds, visible precipitation, convective
illumination, and dust without replacing Atlas, coupling community plugins to a
graphics ABI, or requiring the same GPU API in every Tauri platform WebView.
The factual weather display must survive renderer initialization and device
failure.

## Decision

WyrmGrid will retain MapLibre as the authoritative Atlas renderer and add one
lazy Three.js `WebGPURenderer` behind an application-owned presentation
adapter. The first implementation uses a transparent pointer-free canvas above
MapLibre and a screen-projection bridge derived from the current MapLibre
camera.

The renderer is **WebGPU-preferred, not WebGPU-required**:

- Three.js requests WebGPU and uses its WebGL2 backend when unavailable.
- Compatibility and low-resource runs do not load Three.js.
- Enhanced and Cinematic have explicit visible-cell, geometry, particle,
  ray-march, resolution, and frame-rate ceilings.
- WebGPU may ray-march a host-generated deterministic 3D density texture for a
  bounded subset of cloud, obscuration, and dust cells. The WebGL2 backend uses
  the lower-cost mesh and point-volume representation.
- Atlas may compare MapLibre's public projection and unprojection results to
  fade an anchored decorative cell behind the globe or pitched-map horizon.
  This does not constitute shared terrain depth or volume-wide occlusion.
- Ray-marched volumes may use deterministic screen-stable sample stratification
  to reduce slicing. Frame-varying noise and unbounded temporal history are not
  introduced by this decision.
- Sustained renderer-submission pressure may reduce in-memory quality ceilings
  gradually. It cannot remove factual layers, persist a hardware label, or send
  a performance measurement outside the process.
- MapLibre keeps markers, labels, radar, wind vectors, provenance, selection,
  and interaction in every profile.
- Decorative MapLibre weather remains visible during initialization and is
  hidden only after Three.js reports a ready backend.
- Initialization, render, or graphics-device failure restores the MapLibre
  decoration without removing facts.
- Reduced Motion makes the Three.js scene static. Repeated lightning pulses
  also remain subject to the separate default-on flash-safety preference.

The renderer receives a bounded host-owned scene containing stable identity,
validated coordinates, explicit condition, bounded visual intensity, and
sourced wind. It does not receive raw provider responses, credentials, a
plugin-supplied shader, or authority to infer weather extent. Community plugins
remain out-of-process and data-only.

The dual-canvas implementation does not claim shared depth. Terrain and
building occlusion, insertion between MapLibre layers, and exact volume-wide
globe horizon intersection remain unavailable until implemented through a
supported contract.

If a future MapLibre WebGPU backend exposes a stable shared-device and custom-
layer depth API, the Three.js adapter may move to one WebGPU render graph. The
domain model, application weather views, plugin protocol, settings, and bounded
render scene must not depend on that migration.

## Consequences

WyrmGrid gains cross-platform 3D weather with one presentation dependency and
without replacing the existing map. The dynamic import keeps Three.js out of
Compatibility startup and makes backend failure recoverable. The adapter and
hard budgets keep Three.js details out of the Svelte component and centralize
resource disposal.

The application pays for a second canvas and render pass in detailed profiles.
Map labels may be visually obscured by weather, and weather is not yet hidden
by terrain or buildings. The Three.js WebGPU renderer is still evolving, so
dependency upgrades require frontend type checking, tests, production build,
cross-WebView rendering checks, fallback verification, and dependency audit.
The ray-marched volumes use procedural presentation texture, not fabricated
meteorological extent: their anchors, conditions, and bounded intensity still
come only from the host-owned render scene.
The full implementation and validation process is documented in
[Atlas Three.js weather renderer](../../atlas/weather-renderer.md).

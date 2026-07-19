# Atlas Three.js weather renderer

This document records the implemented MapLibre and Three.js rendering boundary,
the decisions that keep it cross-platform, and the migration path toward a
future shared WebGPU renderer. It supplements the factual and provenance rules
in [Flight plans and weather in Atlas](flight-plan-and-weather.md).

## Current status

Atlas uses two composited renderers for detailed weather:

1. MapLibre GL JS renders the globe, basemap, radar, routes, sourced weather
   markers, labels, wind vectors, selection, and interaction through WebGL2.
2. A transparent Three.js canvas renders source-shaped ray-marched cloud,
   obscuration, and dust volumes plus precipitation particles and lightning
   above the map. Its WebGL2 backend retains bounded mesh and point-volume
   substitutes for the ray-marched effects.
3. Svelte renders controls and accessible factual presentation above both.

The Three.js module is loaded only when Enhanced or Cinematic weather is
selected, a supported phenomenon is visible, and low-resource mode has not
forced Compatibility. `WebGPURenderer` requests WebGPU first and uses its
WebGL2 backend when WebGPU is unavailable. The status displayed in Atlas is the
backend that actually initialized, not a claim inferred from the selected
profile.

This is intentionally **WebGPU-preferred**, not WebGPU-required. Tauri uses the
platform WebView, so a working build cannot assume identical WebGPU support on
Windows, macOS, and Linux.

## Data and authority boundary

```text
validated application weather view
              |
              v
  bounded presentation scene
  (coordinates, condition, intensity,
   sourced wind and stable identity)
              |
       +------+------+
       |             |
       v             v
   MapLibre       Three.js
 facts/labels     decoration only
```

The render-scene builder consumes only host-owned airport and plugin weather
views. It does not receive raw provider JSON, credentials, arbitrary plugin
styles, executable shader code, or a plugin callback. Unknown and clear grid
conditions do not create Three.js cells. Missing airport weather does not
become clear weather.

Visual intensity is bounded to `0..1`. Precipitation greater than the named
visual ceiling increases neither particle count nor geographical extent.
Airport cells take priority over plugin-grid cells when the visible-cell budget
is full. This is a presentation decision only; every factual marker and label
remains in MapLibre.

## Composition and projection

The weather canvas is a pointer-transparent sibling above MapLibre. On every
weather frame, Atlas asks MapLibre to project each validated longitude and
latitude into the current viewport. Atlas then unprojects the screen point and
compares the round-trip coordinate with the source. Points behind the globe or
above a pitched-map horizon resolve onto a different visible surface, allowing
the renderer to fade and reject those decorative cells without using a private
MapLibre transform API. The renderer also samples the bounded visual perimeter
through MapLibre's public project/unproject pair and fades the whole decorative
cell before particles or volume geometry can cross the visible globe edge.
Three.js uses a screen-aligned perspective camera whose target plane preserves
CSS-pixel positions. This matches the perspective view rays required by the
TSL box ray marcher while retaining MapLibre as the geographic camera owner.

This first slice deliberately avoids a world-spanning cloud mesh. It keeps
effects local to evidence and avoids pretending that one sparse report defines
a regional storm field. It also reduces the amount of globe-curvature logic
that must be duplicated while MapLibre still owns a WebGL2 render graph.

This anchor-aware visibility is not shared scene depth. The current dual-canvas
design still cannot share MapLibre's depth buffer. Therefore:

- terrain and buildings do not yet occlude Three.js weather;
- Three.js weather cannot be inserted between individual MapLibre layers;
- map labels may be atmospherically obscured, while Svelte controls and factual
  inspectors remain above the weather; and
- volume-wide horizon intersection and globe-scale geometry remain future work.

These are visible renderer limits, not reasons to invent hidden terrain,
cloud, or strike data.

## Effect implementation

- **Clouds and obscuration:** the WebGPU backend ray-marches a shared 80³
  deterministic, multi-lobed density texture through source-local boxes using
  Three.js TSL. Each cell receives bounded, condition- and intensity-driven
  density, optical thickness, colour, scale, sourced-wind alignment, and
  motion. Total optical thickness is converted to per-sample opacity using the
  active step count, so adaptive quality changes edge smoothness without
  making the cloud disappear. The density field is forced to zero on every
  texture face so the invisible sampling box cannot become a visible slab.
  Per-cell threshold, three-dimensional sample rotation, offset, and aspect
  variation prevent the shared field from appearing as a repeated stamp. Cells
  beyond the volume ceiling, and every WebGL2-backend cell, use the
  deterministic lit-mesh representation.
  The one-time field generation yields between small slice batches so renderer
  startup does not monopolize the UI thread. A deterministic screen-space
  interleaved offset stratifies samples along each view ray, reducing visible
  low-step slicing without adding frame-varying temporal noise.
- **Rain and snow:** bounded GPU-instanced geometry with deterministic
  positions, sourced wind direction, bounded density, and camera-relative
  recycling. The one-sided field begins below its parent volume and tapers at
  its sides, cloud edge, and lower edge.
- **Lightning:** a deterministic local bolt and illumination attached to a
  sourced convective cell. It is not presented as an observed strike.
- **Dust:** WebGPU combines a broad ray-marched brown density volume with
  bounded foreground points. WebGL2 retains the point-volume representation.
  Both are attached only to an explicit dust or sand condition.

Animation stops under Reduced Motion. Flash pulses also require the user to
disable the default Reduce Weather Flashes protection; otherwise lightning is
shown without repeated pulsing.

## Visual-quality baseline and reference

Maintainer captures reviewed on 2026-07-19 confirmed that the horizon-clipping
repair removed the hard sampling-box wedges and off-globe particle spill. The
remaining WebGPU clouds were nevertheless too small, dark, translucent, and
elliptical. Reusing one elongated density field made separate weather cells
look like rotated copies of the same grey smoke capsule.

The visual-quality pass uses the upstream Three.js
[`webgpu_volume_cloud`](https://threejs.org/examples/webgpu_volume_cloud.html)
demonstration as a renderer reference, not as a complete weather-system design.
Its source demonstrates a 128-cubed 3D noise texture, 100 ray samples, a lower
density threshold, materially stronger sample opacity, density-gradient
lighting, early alpha termination, and developer controls for threshold,
opacity, transition range, and sample count. These choices explain its solid,
internally shaded silhouette. WyrmGrid must adapt rather than copy them because
it renders several bounded cells over a geographic camera and must keep its
appearance stable while adaptive quality changes the sample count.

The pass therefore follows these rules:

- keep density, extinction, edge softness, lighting, scale, and variation as
  named renderer configuration rather than scattered shader literals;
- use an irregular multi-lobed envelope so mesh scale does not stretch a
  spherical density field into a capsule;
- vary the three-dimensional sample orientation, scale, and offset per stable
  cell identity so a shared texture does not become a repeated stamp, while
  aligning the broad screen silhouette with sourced wind when it is available;
- derive per-sample absorption from a total optical thickness and the current
  ray-step budget, so lowering quality changes smoothness rather than making
  clouds disappear;
- place precipitation in a one-sided field below its parent cloud and taper it
  at the sides and lower edge;
- reserve raw shader parameters for a deterministic developer gallery; user
  settings remain Compatibility, Enhanced, Cinematic, effect toggles, Reduced
  Motion, and flash protection; and
- do not infer a regional storm footprint from a point sample. Future
  world-scale systems require an explicit host-owned coverage or cell-boundary
  contract.

### Visual acceptance criteria

The deterministic reference scenes must demonstrate all of the following:

1. no hard box face, wedge, or off-globe particle spill;
2. cloud silhouettes with multiple lobes and no dominant capsule outline;
3. visibly brighter illuminated regions and darker internal or lower regions;
4. several simultaneous cells that do not read as rotated copies;
5. rain and snow beginning below, rather than surrounding, the cloud body;
6. Enhanced and Cinematic retaining the same broad appearance while Cinematic
   improves edge detail, volume count, and particle density; and
7. Compatibility and renderer failure continuing to preserve the factual
   MapLibre weather presentation.

The isolated developer gallery is a visual calibration aid, not a second
source of renderer rules. Its scenes and controls consume the same exported
appearance configuration and scene-building helpers used by Atlas. Hardware
captures remain necessary because shader output and WebView WebGPU support
cannot be proven by unit tests alone.

The gallery is available automatically from Diagnostics in development builds.
A packaged build exposes it only when WyrmGrid is deliberately launched with
`--weather-gallery`; that flag opens the isolated gallery instead of the normal
workspace and suppresses automatic simulator-sidecar startup. Direct browser
navigation, query parameters, or stored preferences cannot enable it in a
packaged build. Calibration values remain in memory for that gallery session
and never update the supported user graphics preferences.

## Profiles and hard budgets

| Profile       | Pixel ratio ceiling | Visible cells | WebGPU volume cells | Ray steps | Fallback cloud puffs/cell | Precipitation instances/cell | Dust points/cell |
| ------------- | ------------------- | ------------- | ------------------- | --------- | ------------------------- | ---------------------------- | ---------------- |
| Compatibility | 1.0                 | 0             | 0                   | 0         | 0                         | 0                            | 0                |
| Enhanced      | 1.0                 | 48            | 8                   | 32        | 4                         | 18                           | 24               |
| Cinematic     | 1.5                 | 96            | 16                  | 64        | 7                         | 36                           | 48               |

The ceilings are named in one presentation module and covered by unit tests.
They are initial safety limits, not measured hardware classifications. Future
changes require frame-time, memory-estimate, device-loss, and cross-WebView
evidence. A small window is never treated as proof of a weak GPU.

The existing profile frame caps remain 20 FPS for Enhanced and 30 FPS for
Cinematic. MapLibre may render independently while the map moves; the weather
renderer reprojects its anchors from the current map camera on every weather
frame.

### Adaptive pressure levels

Within Enhanced and Cinematic, an in-memory controller watches Three.js command
submission time. This is a conservative main-thread pressure signal, **not a
GPU-time or hardware classification measurement**. A single slow frame, shader
compilation, application suspension, or invalid timer sample cannot change the
level.

After sustained pressure the controller moves from Full to Balanced and then
Minimum. Each transition reduces volume cells and ray steps first, followed by
visible-cell, particle, fallback-geometry, and pixel-ratio ceilings. It never
removes factual MapLibre weather. Recovery requires a much longer healthy
window and proceeds one level at a time. The level resets when the user changes
profile or the renderer is recreated; it is neither persisted nor reported as
telemetry.

## Lifecycle and failure handling

The renderer follows one authoritative lifecycle:

1. Build a stable scene from currently visible host-owned weather views.
2. Keep MapLibre's decorative weather layers visible while Three.js loads.
3. Dynamically import Three.js and asynchronously initialize
   `WebGPURenderer`.
4. Hide only the duplicate decorative MapLibre layers after Three.js reports a
   ready backend. Markers, labels, wind vectors, radar, and interactions stay.
5. Dispose scene geometry, node materials, the shared 3D density texture,
   particle buffers, and the renderer when detailed weather is no longer
   requested or Atlas unmounts.
6. On initialization, render, or device loss, restore the MapLibre decoration
   and report the fallback state. Facts never disappear with the renderer.

A failed profile is not retried continuously. Changing away from the profile
and back permits a deliberate retry; new application releases may add bounded
automatic recovery after cross-platform evidence exists.

## Code ownership

The implementation is intentionally kept out of Svelte event handlers:

- `weather/renderer/weatherRenderScene.ts` builds the bounded presentation
  scene;
- `weather/renderer/quality.ts` owns resource ceilings;
- `weather/renderer/adaptiveQuality.ts` owns pressure transitions;
- `weather/renderer/projectionVisibility.ts` and `surfaceClipping.ts` own the
  public-projection horizon checks;
- `weather/renderer/volumeDensity.ts` generates the bounded procedural 3D
  density field;
- `weather/renderer/volumeAppearance.ts` owns the named optical and colour
  presets shared by Atlas and the gallery;
- `weather/renderer/precipitationLayout.ts` owns the bounded, one-sided rain
  and snow layout beneath a weather cell;
- `weather/renderer/deterministic.ts` and `volumeVariation.ts` provide stable,
  cell-specific visual variation;
- `weather/renderer/weatherGallery.ts` builds deterministic reference scenes,
  while `weatherGalleryAccess.ts` owns the development-build and startup-flag
  access rule;
- `weather/renderer/threeWeatherRenderer.ts` owns Three.js resources and
  animation;
- `weather/renderer/types.ts` is the renderer adapter contract; and
- `routes/weather-gallery/+page.svelte` presents the deliberately isolated
  developer gallery without owning renderer rules; and
- `AtlasMap.svelte` owns only composition, visibility, camera projection, and
  lifecycle delegation.

Community plugins continue to publish data-only bounded weather products. They
never receive the MapLibre map, Three.js scene, renderer, GPU device, shaders,
or canvas.

## Validation

The lowest presentation layer tests cover:

- absence and unknown weather creating no render cells;
- explicit airport and plugin conditions producing stable cells;
- intensity clamping and wind conversion;
- Compatibility allocating no Three.js resources; and
- Enhanced remaining below Cinematic ceilings;
- adaptive degradation, slow recovery, and invalid-sample rejection; and
- deterministic density generation, edge tapering, and allocation bounds;
- optical-thickness invariance, multi-lobed density proportions, bounded
  calibration overrides, precipitation placement and tapering, stable gallery
  scenes, and gallery access gating;
- projection round-trip visibility, antimeridian equivalence, horizon fading,
  and invalid-coordinate rejection.

Routine frontend gates remain Svelte type checking, the complete Vitest suite,
Prettier, and the production build. Cross-platform release validation must also
exercise WebGPU, Three.js WebGL2 fallback, initialization failure, device loss,
Reduced Motion, low-resource mode, and profile changes in the supported Tauri
WebViews.

## Future migration

MapLibre is developing a WebGPU backend, but WyrmGrid does not assume that it
will expose a supported custom-renderer device and depth contract. If it does,
the renderer adapter may move to one shared canvas, GPU device, command stream,
and depth texture. That would allow terrain occlusion and correct placement
between map layers without changing application weather models or user
preferences.

If MapLibre does not expose that contract, the dual-canvas renderer remains a
supported architecture. Future improvements can still add stationary-camera
temporal accumulation, compute-assisted particle updates, measured GPU timing
and VRAM budgets, volume-wide horizon intersection, and physically richer
lighting behind the same bounded scene and failure-safe adapter.

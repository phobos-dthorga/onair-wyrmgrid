# Display and performance launch options

WyrmGrid adapts its workspace to the available window rather than assuming that
a small display is a slow computer. A compact resolution may belong to an older
laptop, a powerful handheld gaming PC, a portrait display, a remote desktop
session, or an accessibility setup. Display density and computing performance
are therefore separate choices.

## Automatic display adaptation

The desktop changes presentation from the actual WebView viewport:

- at 900 pixels wide or less, Atlas, Jobs, and Dispatch use a narrow, vertically
  scrollable layout;
- at 720 pixels high or less, navigation and workspace spacing are shortened
  without assuming that the window is narrow;
- wider and taller windows retain the standard three-column workspace.

The window may be resized down to 640 × 560. At very small sizes some complex
workspaces require vertical or horizontal scrolling, but their controls remain
reachable. WyrmGrid does not change data behaviour, simulator telemetry,
privacy boundaries, or plugin permissions in any presentation mode.

## Command-line overrides

Pass one of these switches to the WyrmGrid executable when automatic adaptation
is not enough:

| Switch            | Effect                                                                                                      |
| ----------------- | ----------------------------------------------------------------------------------------------------------- |
| `--no-launch-art` | Uses the lightweight gradient launch screen. The packaged hero image is not mounted, requested, or decoded. |
| `--compact-ui`    | Forces the narrow, single-column workspace even on a larger display.                                        |
| `--low-resource`  | Enables both switches above and also removes decorative blur, shadows, transitions, and animation.          |

For example, from PowerShell:

```powershell
& "C:\Program Files\OnAir WyrmGrid\OnAir WyrmGrid.exe" --compact-ui
& "C:\Program Files\OnAir WyrmGrid\OnAir WyrmGrid.exe" --low-resource
```

The installation path depends on the package and choices made during install.
A Windows shortcut can make an option persistent: copy the normal shortcut,
open **Properties**, and append the switch after the quoted executable path in
the **Target** field.

Use `--compact-ui` for layout preference or constrained screen space. Use
`--low-resource` when reducing decorative graphics work is also desirable—for
example during remote play, on battery power, or alongside a demanding
simulator. A low screen resolution alone is not a reason to select
`--low-resource`.

The switches are local presentation preferences. They are not transmitted to
OnAir, simulator providers, plugins, or diagnostic telemetry.

## Atlas weather profiles

Atlas weather has a separate rendering preference because window size and
graphics capability are independent. **Enhanced** is the default and presents
sourced airport METAR conditions plus approved global-model samples as a
GPU-rendered atmosphere, with wind vectors and restrained condition motion.
**Cinematic** raises the limits for ray-marched 3D cloud and dust density,
denser rain or snow particles, and convective illumination on capable GPUs.
Enhanced and Cinematic use a lazily loaded Three.js renderer that prefers
WebGPU. Its WebGL2 fallback substitutes layered cloud meshes and dust points for
ray-marched volumes. **Compatibility** keeps the same facts with conservative
static markers and does not load Three.js. Approved RADAR tiles remain
host-rendered in every profile. Atlas normally loops the received recent frames
and shows their source time; Compatibility under `--low-resource` and the
operating system's Reduced Motion preference keep the newest frame static.

Choose the profile in **Settings > Weather graphics**. Cloud depth, visible
precipitation, lightning, and dust can each be disabled independently. These
controls affect presentation only: disabling visible rain does not remove the
sourced rain condition or radar product.

Atlas reports the renderer that actually started: Three.js WebGPU, Three.js
WebGL2 fallback, or MapLibre fallback. A renderer initialization or graphics-
device failure restores the MapLibre effect automatically; weather markers,
labels, radar, wind, timestamps, and provenance do not depend on the Three.js
canvas. The current separate canvases do not share terrain depth, so detailed
weather may cover map labels and is not yet occluded by terrain or buildings.
WyrmGrid does compare each weather anchor with MapLibre's projection round trip,
so effects fade behind the globe or a pitched-map horizon instead of remaining
painted over empty space. This anchor check is not terrain depth. See the
[weather renderer design](../atlas/weather-renderer.md) for this limit and the
future shared-WebGPU path.

Detailed profiles also adapt within their selected ceiling. If renderer
submission remains expensive, WyrmGrid gradually reduces volume count, ray
steps, particles, and resolution; it restores them only after a long healthy
period. This temporary level is not saved as a judgement about the device and
never changes the weather facts or the user's selected profile.

The airport atmosphere remains local to its report. The global model layer
renders only the coarse points WyrmGrid requested, while radar uses the
provider's bounded source tiles. WyrmGrid does not turn sparse METARs or model
samples into invented observations between stations. A METAR thunderstorm
supports a station-local storm symbol and illumination, not an invented strike
coordinate. Exact strike visualisation remains unavailable until a provider
supplies a bounded lightning product.

Atlas also provides two independent operational-layer switches:

- **Day and night** calculates the current solar position from UTC and shades
  civil, nautical, astronomical twilight, and night. In Historical mode it
  uses the selected Hoard time rather than the present.
- **Weather support zones** places soft indicative rings behind airport
  effects, compact sample patches behind complete regular forecast grids, and
  outlines around the exact RADAR tiles WyrmGrid received. Airport rings are
  not measured storm boundaries; compact grid patches identify the nearest
  validated sample while gaps remain unknown; and RADAR outlines show tile
  footprint rather than station range or proof of measurements in every pixel.
  Each condition also has a distinct
  repeating pattern—such as slanted rain strokes, snow stipple, storm
  chevrons, or dust crosshatching—so colour is not the only way to distinguish
  zones.

These layers can be hidden without changing sourced weather, renderer quality,
provider refresh, or saved graphics preferences. Their evidence and the future
eclipse-path design are documented in the
[Atlas daylight and weather coverage contract](../atlas/daylight-weather-coverage-and-eclipses.md).

When the RainViewer layer is active, the RADAR history panel identifies the
provider, selected time, and frame position and offers play, pause, previous,
and next controls. Neutral-grey pixels are explicitly **no RADAR coverage**;
they do not mean clear weather. A planned route may also carry a wider coloured
global-model corridor. Its Dispatch card lists each coarse checkpoint and the
distance to the model point supporting it. Dashed neutral sections have no
nearby model support and are not silently joined or filled.

**Reduce weather flashes** is enabled by default. Turning it off requires an
explicit photosensitivity confirmation. The operating system's Reduce Motion
preference always keeps detailed weather static and flash-free, regardless of
the saved profile or flash choice.

`--low-resource` will force the Compatibility profile for that run. It will not
silently change the saved preference, and a low display resolution alone will
never be treated as evidence of a weak GPU. See the
[Atlas flight-plan and weather contract](../atlas/flight-plan-and-weather.md)
for the profiles and safeguards.

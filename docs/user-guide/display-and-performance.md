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

## Planned Atlas weather profiles

Atlas weather will use a separate rendering preference because window size and
graphics capability are independent. **Prefer compatibility weather rendering**
will be enabled by default. It will retain route, time, provenance, and hazard
facts while using simpler symbols, shapes, and animation. Users with suitable
hardware may opt into GPU-enhanced wind, imagery, and time animation after
capability detection.

**Reduce flashes** will remain enabled by default independently of the selected
profile. Enhanced or cinematic weather will not disable it automatically.
Turning it off will require an explicit photosensitivity warning and user
confirmation; accepting the Application Terms alone will not enable stronger
flashing effects.

`--low-resource` will force the Compatibility profile for that run. It will not
silently change the saved preference, and a low display resolution alone will
never be treated as evidence of a weak GPU. See the
[Atlas flight-plan and weather contract](../atlas/flight-plan-and-weather.md)
for the planned profiles and safeguards.

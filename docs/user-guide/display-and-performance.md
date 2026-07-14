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

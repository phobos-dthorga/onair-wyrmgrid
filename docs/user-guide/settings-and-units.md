# Settings and measurement units

## Moving through dialogs

WyrmGrid remembers the path through nested dialogs. Closing a child returns to
the dialog that opened it—for example, **Settings → Encrypted data & backups →
Open-source licences** returns first to Encrypted data & backups, then to
Settings. Escape, the close button, and the dialog's ordinary Cancel or Done
action follow the same rule.

Independent dialogs opened from the main workspace start a fresh path, so an
older Settings history cannot unexpectedly reappear. The shared navigation
stack also applies to future nested children opened from Simulator, Hoard,
Forge, Diagnostics, or provider dialogs.

Open **Settings** from WyrmGrid's top navigation to manage measurement units,
themes, language packs, and privacy choices from one place.

## Simulator provider launch

The **Provider launch** section remembers which trusted simulator sidecar to
use. **Start this provider with WyrmGrid** is off by default. When enabled,
WyrmGrid launches only that approved provider in the background; it may wait
harmlessly until MSFS is available.

This preference does not start MSFS, begin flight recording, change simulator
state, or grant telemetry access to community plugins. Disable it at any time
to return to explicit manual provider start on future launches.

## Flight recording

Open **Simulator**, wait for a fresh live aircraft snapshot, and select
**Start recording**. This creates a local session and stores translated,
validated one-hertz samples until **Stop recording** is selected. Starting the
provider alone never starts a recording.

**Start recordings automatically when flight begins** is off by default. When
enabled, WyrmGrid requires two fresh, increasing, unpaused samples that report
the aircraft directly as airborne. It never substitutes altitude or speed as
takeoff evidence. Automatic sessions can close after the simulator reports a
continuous on-ground settling period; a telemetry gap restarts that period.
The settling time and automatic stop can be changed independently in Settings.

Recent sessions appear in the same dialog. Selecting one opens altitude and
speed graphs for a window of 600 exact samples. **Older** and **Newer** move
between exact windows without downsampling or silently omitting rows. A visible
break means samples were missing; WyrmGrid does not draw a continuous line
across that interval.
Changing aircraft or registration interrupts the active session so facts from
two aircraft are not silently combined.

The same retained sessions and graphs are available from **Hoard → Flight
recordings**. Hoard is the durable browsing surface; the Simulator bridge keeps
capture controls close to the live connection. Both surfaces use one shared
view and the same unit preferences.

**Keep completed recordings** defaults to 30 days and can be changed to 7, 90,
or 365 days in Settings. Expired completed and interrupted sessions are pruned
locally unless pinned. Pinning protects a recording from automatic retention;
it does not prevent explicit deletion. Each inactive session can be deleted
individually, or all inactive sessions can be deleted together. Stop an active
recording before deleting it.

When a SimBrief plan is present before capture starts, WyrmGrid stores a
sanitized plan snapshot with the recording. Hoard then compares only supported
like-for-like facts such as planned and recorded duration, distance, peak
altitude, fuel use, airport proximity, and registration. Unsupported or
ambiguous comparisons stay **Unavailable** rather than being inferred.

**Export JSON** includes the complete exact sample series, events, stored plan
snapshot, and calculated comparison. **Export CSV** contains the complete
exact sample series. Exported files are ordinary plaintext files and are not
protected by Hoard's database encryption, so store and share them accordingly.
Recorded sessions may include precise positions and operational measurements;
they are not automatically exposed to community plugins.

If a connected provider stops supplying samples, WyrmGrid marks the stream
stale after five seconds and hides the old snapshot. Waiting for MSFS,
reconnecting after MSFS closes, stale telemetry, and provider failure remain
distinct states so an old aircraft position is never presented as live.

## Independent unit choices

Measurement categories are saved independently. Selecting litres for fuel does
not change altitude, speed, aircraft weight, simulator configuration, or the
aircraft itself.

| Category             | Available presentation units                                                |
| -------------------- | --------------------------------------------------------------------------- |
| Altitude             | feet, metres                                                                |
| Air and ground speed | knots, miles per hour, kilometres per hour, metres per second               |
| Aircraft weight      | pounds, kilograms                                                           |
| Fuel quantity        | pounds by weight, kilograms by weight, US gallons, Imperial gallons, litres |

Headings and geographic coordinates remain degrees because none of the unit
choices replaces angular measurements.

The Aviation, Imperial, Metric, and SI buttons are convenience presets. Applying
a preset fills the individual controls, which can then be changed independently
before saving. The default Aviation presentation preserves WyrmGrid's original
feet, knots, and pounds display.

## Fuel facts and conversions

WyrmGrid distinguishes fuel mass from fuel volume. It converts the simulator's
reported US-gallon volume to Imperial gallons or litres, and its reported pound
weight to kilograms. It does not infer one dimension from the other using an
assumed fuel density. If the simulator snapshot lacks the source dimension
needed by the selected fuel unit, WyrmGrid displays **Unavailable**.

This matters because fuel density varies with fuel type and conditions. A
presentation preference must not turn an absent external fact into an invented
one.

## Scope and privacy

Unit preferences are stored locally in WyrmGrid's SQLite database. They change
only presentation formatting. WyrmGrid does not send them to OnAir, MSFS,
SimConnect providers, community plugins, or diagnostic telemetry.

The Bridge protocol and telemetry snapshot retain their canonical versioned
fields—currently feet, knots, pounds, US gallons, and degrees. Plugins therefore
receive stable facts rather than values whose meaning changes with the desktop
user's display choices.

## Implementation notes

The append-only `0006_display_preferences.sql` migration stores one typed choice
per measurement category. `DisplaySettingsService` owns validation and
persistence, while frontend presentation helpers perform finite, deterministic
conversions. Adding a new displayed category should add a typed preference,
storage constraint, conversion boundary tests, interface control, and this
documentation together. Existing migration files must not be edited.

The append-only `0007_simulator_preferences.sql` migration stores the selected
provider ID and the default-off launch preference. `SimulatorSettingsService`
validates the selection against installed provider registrations; the frontend
cannot supply an arbitrary executable path.

The append-only `0008_simulator_recordings.sql` migration owns the separate
recording-retention preference, session identity and provenance, and bounded
translated samples. `SimulatorRecordingService` owns start, stop, interruption,
gap detection, pruning, and deletion; Svelte handlers only delegate actions and
format canonical measurements into the user's selected display units.

The append-only `0010_simulator_evidence.sql` migration adds automation
preferences, direct lifecycle facts, recording events, pins, and sanitized plan
associations without rewriting the released recording migration.

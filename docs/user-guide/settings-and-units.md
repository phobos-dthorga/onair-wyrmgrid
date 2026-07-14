# Settings and measurement units

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

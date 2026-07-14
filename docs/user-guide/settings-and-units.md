# Settings and measurement units

Open **Settings** from WyrmGrid's top navigation to manage measurement units,
themes, language packs, and privacy choices from one place.

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

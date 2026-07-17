export type AltitudeUnit = "feet" | "metres";
export type SpeedUnit =
  "knots" | "miles_per_hour" | "kilometres_per_hour" | "metres_per_second";
export type WeightUnit = "pounds" | "kilograms";
export type FuelUnit =
  "pounds" | "kilograms" | "us_gallons" | "imperial_gallons" | "litres";

export type DisplayPreferences = {
  altitude_unit: AltitudeUnit;
  speed_unit: SpeedUnit;
  weight_unit: WeightUnit;
  fuel_unit: FuelUnit;
  responsive_surfaces: boolean;
};

export const aviationDisplayPreferences: DisplayPreferences = {
  altitude_unit: "feet",
  speed_unit: "knots",
  weight_unit: "pounds",
  fuel_unit: "pounds",
  responsive_surfaces: true,
};

export const displayPresets = {
  aviation: aviationDisplayPreferences,
  imperial: {
    altitude_unit: "feet",
    speed_unit: "miles_per_hour",
    weight_unit: "pounds",
    fuel_unit: "imperial_gallons",
    responsive_surfaces: true,
  },
  metric: {
    altitude_unit: "metres",
    speed_unit: "kilometres_per_hour",
    weight_unit: "kilograms",
    fuel_unit: "litres",
    responsive_surfaces: true,
  },
  si: {
    altitude_unit: "metres",
    speed_unit: "metres_per_second",
    weight_unit: "kilograms",
    fuel_unit: "kilograms",
    responsive_surfaces: true,
  },
} as const satisfies Record<string, DisplayPreferences>;

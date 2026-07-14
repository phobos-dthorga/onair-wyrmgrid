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
};

export const aviationDisplayPreferences: DisplayPreferences = {
  altitude_unit: "feet",
  speed_unit: "knots",
  weight_unit: "pounds",
  fuel_unit: "pounds",
};

export const displayPresets = {
  aviation: aviationDisplayPreferences,
  imperial: {
    altitude_unit: "feet",
    speed_unit: "miles_per_hour",
    weight_unit: "pounds",
    fuel_unit: "imperial_gallons",
  },
  metric: {
    altitude_unit: "metres",
    speed_unit: "kilometres_per_hour",
    weight_unit: "kilograms",
    fuel_unit: "litres",
  },
  si: {
    altitude_unit: "metres",
    speed_unit: "metres_per_second",
    weight_unit: "kilograms",
    fuel_unit: "kilograms",
  },
} as const satisfies Record<string, DisplayPreferences>;

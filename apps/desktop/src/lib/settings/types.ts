export type AltitudeUnit = "feet" | "metres";
export type SpeedUnit =
  "knots" | "miles_per_hour" | "kilometres_per_hour" | "metres_per_second";
export type WeightUnit = "pounds" | "kilograms";
export type FuelUnit =
  "pounds" | "kilograms" | "us_gallons" | "imperial_gallons" | "litres";
export type WeatherRenderingProfile =
  "compatibility" | "enhanced" | "cinematic";

export type DisplayPreferences = {
  altitude_unit: AltitudeUnit;
  speed_unit: SpeedUnit;
  weight_unit: WeightUnit;
  fuel_unit: FuelUnit;
  responsive_surfaces: boolean;
  weather_rendering_profile: WeatherRenderingProfile;
  weather_cloud_effects: boolean;
  weather_precipitation_effects: boolean;
  weather_lightning_effects: boolean;
  weather_dust_effects: boolean;
  reduce_weather_flashes: boolean;
};

export type WeatherGraphicsPreferences = Pick<
  DisplayPreferences,
  | "weather_rendering_profile"
  | "weather_cloud_effects"
  | "weather_precipitation_effects"
  | "weather_lightning_effects"
  | "weather_dust_effects"
  | "reduce_weather_flashes"
>;

export const aviationDisplayPreferences: DisplayPreferences = {
  altitude_unit: "feet",
  speed_unit: "knots",
  weight_unit: "pounds",
  fuel_unit: "pounds",
  responsive_surfaces: true,
  weather_rendering_profile: "enhanced",
  weather_cloud_effects: true,
  weather_precipitation_effects: true,
  weather_lightning_effects: true,
  weather_dust_effects: true,
  reduce_weather_flashes: true,
};

export const displayPresets = {
  aviation: aviationDisplayPreferences,
  imperial: {
    ...aviationDisplayPreferences,
    altitude_unit: "feet",
    speed_unit: "miles_per_hour",
    weight_unit: "pounds",
    fuel_unit: "imperial_gallons",
  },
  metric: {
    ...aviationDisplayPreferences,
    altitude_unit: "metres",
    speed_unit: "kilometres_per_hour",
    weight_unit: "kilograms",
    fuel_unit: "litres",
  },
  si: {
    ...aviationDisplayPreferences,
    altitude_unit: "metres",
    speed_unit: "metres_per_second",
    weight_unit: "kilograms",
    fuel_unit: "kilograms",
  },
} as const satisfies Record<string, DisplayPreferences>;

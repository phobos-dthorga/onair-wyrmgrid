import { describe, expect, it } from "vitest";
import { aviationDisplayPreferences } from "./types";
import { validatePreferences } from "./client";

const legacyPreferences = {
  altitude_unit: "metres",
  speed_unit: "metres_per_second",
  weight_unit: "kilograms",
  fuel_unit: "litres",
  responsive_surfaces: false,
};

describe("display preference validation", () => {
  it("upgrades a saved pre-weather preference to Enhanced rendering", () => {
    expect(validatePreferences(legacyPreferences)).toEqual({
      ...legacyPreferences,
      weather_rendering_profile: "enhanced",
      weather_cloud_effects: true,
      weather_precipitation_effects: true,
      weather_lightning_effects: true,
      weather_dust_effects: true,
      reduce_weather_flashes: true,
    });
  });

  it("accepts cinematic weather with independently managed effects", () => {
    expect(
      validatePreferences({
        ...legacyPreferences,
        weather_rendering_profile: "cinematic",
        weather_cloud_effects: true,
        weather_precipitation_effects: false,
        weather_lightning_effects: false,
        weather_dust_effects: true,
        reduce_weather_flashes: true,
      }),
    ).toEqual({
      ...legacyPreferences,
      weather_rendering_profile: "cinematic",
      weather_cloud_effects: true,
      weather_precipitation_effects: false,
      weather_lightning_effects: false,
      weather_dust_effects: true,
      reduce_weather_flashes: true,
    });
  });

  it("fails closed when a weather effect preference is not boolean", () => {
    expect(
      validatePreferences({
        ...legacyPreferences,
        weather_cloud_effects: "yes",
      }),
    ).toEqual(aviationDisplayPreferences);
  });
});

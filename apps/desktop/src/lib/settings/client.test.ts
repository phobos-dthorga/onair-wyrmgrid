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
    });
  });

  it("fails closed to known defaults for an unsupported weather profile", () => {
    expect(
      validatePreferences({
        ...legacyPreferences,
        weather_rendering_profile: "cinematic",
      }),
    ).toEqual(aviationDisplayPreferences);
  });
});

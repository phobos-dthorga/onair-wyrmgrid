import { describe, expect, it } from "vitest";
import type { WeatherGraphicsPreferences } from "$lib/settings/types";
import {
  lightningFlashOpacity,
  resolveWeatherGraphicsPolicy,
} from "./graphics";

const cinematic: WeatherGraphicsPreferences = {
  weather_rendering_profile: "cinematic",
  weather_cloud_effects: true,
  weather_precipitation_effects: true,
  weather_lightning_effects: true,
  weather_dust_effects: true,
  reduce_weather_flashes: false,
};

describe("weather graphics policy", () => {
  it("enables the complete cinematic treatment when explicitly selected", () => {
    expect(resolveWeatherGraphicsPolicy(cinematic, false, false)).toEqual({
      profile: "cinematic",
      atmosphere: true,
      clouds: true,
      precipitation: true,
      lightning: true,
      dust: true,
      animation: true,
      lightningFlashes: true,
      particleScale: 1,
      frameRate: 30,
    });
  });

  it("forces a marker-only policy without rewriting the saved profile", () => {
    const policy = resolveWeatherGraphicsPolicy(cinematic, true, false);
    expect(policy).toMatchObject({
      profile: "compatibility",
      atmosphere: false,
      clouds: false,
      precipitation: false,
      lightning: false,
      dust: false,
      animation: false,
      lightningFlashes: false,
    });
    expect(cinematic.weather_rendering_profile).toBe("cinematic");
  });

  it("keeps detailed phenomena static and flash-free under Reduce Motion", () => {
    expect(resolveWeatherGraphicsPolicy(cinematic, false, true)).toMatchObject({
      profile: "cinematic",
      clouds: true,
      precipitation: true,
      lightning: true,
      dust: true,
      animation: false,
      lightningFlashes: false,
    });
  });

  it("honours independent effect switches", () => {
    expect(
      resolveWeatherGraphicsPolicy(
        {
          ...cinematic,
          weather_cloud_effects: false,
          weather_lightning_effects: false,
        },
        false,
        false,
      ),
    ).toMatchObject({
      clouds: false,
      lightning: false,
      lightningFlashes: false,
    });
  });

  it("uses two bounded flashes in each long illumination cycle", () => {
    expect(lightningFlashOpacity(0)).toBe(0.62);
    expect(lightningFlashOpacity(170)).toBe(0.34);
    expect(lightningFlashOpacity(500)).toBe(0.025);
    expect(lightningFlashOpacity(6_400)).toBe(0.62);
  });
});

import { describe, expect, it } from "vitest";
import type { WeatherGraphicsPolicy } from "$lib/weather/graphics";
import {
  adaptWeatherRenderBudget,
  resolveWeatherRenderBudget,
} from "./quality";

const enhanced: WeatherGraphicsPolicy = {
  profile: "enhanced",
  atmosphere: true,
  clouds: true,
  precipitation: true,
  lightning: true,
  dust: true,
  animation: true,
  lightningFlashes: false,
  particleScale: 0.58,
  frameRate: 20,
};

describe("Three.js weather rendering budgets", () => {
  it("keeps Enhanced below the Cinematic resource ceiling", () => {
    const enhancedBudget = resolveWeatherRenderBudget(enhanced);
    const cinematicBudget = resolveWeatherRenderBudget({
      ...enhanced,
      profile: "cinematic",
      particleScale: 1,
      frameRate: 30,
    });

    expect(enhancedBudget.maximumPixelRatio).toBe(1);
    expect(cinematicBudget.maximumPixelRatio).toBeGreaterThan(
      enhancedBudget.maximumPixelRatio,
    );
    expect(cinematicBudget.maximumCells).toBeGreaterThan(
      enhancedBudget.maximumCells,
    );
    expect(cinematicBudget.maximumVolumeCells).toBeGreaterThan(
      enhancedBudget.maximumVolumeCells,
    );
    expect(cinematicBudget.volumeRaymarchSteps).toBeGreaterThan(
      enhancedBudget.volumeRaymarchSteps,
    );
  });

  it("allocates no Three.js effects to Compatibility", () => {
    expect(
      resolveWeatherRenderBudget({
        ...enhanced,
        profile: "compatibility",
        atmosphere: false,
        animation: false,
        frameRate: 0,
      }).maximumCells,
    ).toBe(0);
  });

  it("reduces expensive volume and particle work before disabling weather", () => {
    const full = resolveWeatherRenderBudget({
      ...enhanced,
      profile: "cinematic",
    });
    const balanced = adaptWeatherRenderBudget(full, "balanced");
    const minimum = adaptWeatherRenderBudget(full, "minimum");

    expect(balanced.maximumVolumeCells).toBeLessThan(full.maximumVolumeCells);
    expect(minimum.maximumVolumeCells).toBeLessThan(
      balanced.maximumVolumeCells,
    );
    expect(minimum.volumeRaymarchSteps).toBeGreaterThanOrEqual(12);
    expect(minimum.precipitationParticlesPerCell).toBeGreaterThan(0);
  });
});

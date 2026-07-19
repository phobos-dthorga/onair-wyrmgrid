import type { WeatherGraphicsPolicy } from "$lib/weather/graphics";

export type WeatherRenderBudget = {
  maximumPixelRatio: number;
  maximumCells: number;
  maximumVolumeCells: number;
  volumeRaymarchSteps: number;
  cloudPuffsPerCell: number;
  precipitationParticlesPerCell: number;
  dustParticlesPerCell: number;
};

export type AdaptiveWeatherQuality = "full" | "balanced" | "minimum";

const COMPATIBILITY_BUDGET: WeatherRenderBudget = {
  maximumPixelRatio: 1,
  maximumCells: 0,
  maximumVolumeCells: 0,
  volumeRaymarchSteps: 0,
  cloudPuffsPerCell: 0,
  precipitationParticlesPerCell: 0,
  dustParticlesPerCell: 0,
};

const ENHANCED_BUDGET: WeatherRenderBudget = {
  maximumPixelRatio: 1,
  maximumCells: 48,
  maximumVolumeCells: 8,
  volumeRaymarchSteps: 32,
  cloudPuffsPerCell: 4,
  precipitationParticlesPerCell: 18,
  dustParticlesPerCell: 24,
};

const CINEMATIC_BUDGET: WeatherRenderBudget = {
  maximumPixelRatio: 1.5,
  maximumCells: 96,
  maximumVolumeCells: 16,
  volumeRaymarchSteps: 64,
  cloudPuffsPerCell: 7,
  precipitationParticlesPerCell: 36,
  dustParticlesPerCell: 48,
};

export function resolveWeatherRenderBudget(
  policy: WeatherGraphicsPolicy,
): WeatherRenderBudget {
  if (!policy.atmosphere || policy.profile === "compatibility") {
    return COMPATIBILITY_BUDGET;
  }
  return policy.profile === "cinematic" ? CINEMATIC_BUDGET : ENHANCED_BUDGET;
}

function scaledCount(value: number, scale: number, minimum: number): number {
  if (value === 0) return 0;
  return Math.max(minimum, Math.floor(value * scale));
}

export function adaptWeatherRenderBudget(
  budget: WeatherRenderBudget,
  quality: AdaptiveWeatherQuality,
): WeatherRenderBudget {
  if (quality === "full" || budget.maximumCells === 0) return budget;
  const minimum = quality === "minimum";
  const scale = minimum ? 0.5 : 0.75;
  return {
    maximumPixelRatio: Math.min(budget.maximumPixelRatio, minimum ? 1 : 1.25),
    maximumCells: scaledCount(budget.maximumCells, scale, 1),
    maximumVolumeCells: scaledCount(
      budget.maximumVolumeCells,
      minimum ? 0.375 : 0.625,
      2,
    ),
    volumeRaymarchSteps: scaledCount(
      budget.volumeRaymarchSteps,
      minimum ? 0.55 : 0.75,
      12,
    ),
    cloudPuffsPerCell: scaledCount(budget.cloudPuffsPerCell, scale, 2),
    precipitationParticlesPerCell: scaledCount(
      budget.precipitationParticlesPerCell,
      scale,
      8,
    ),
    dustParticlesPerCell: scaledCount(budget.dustParticlesPerCell, scale, 10),
  };
}

import type { WeatherRenderEffect } from "./weatherRenderScene";

export type WeatherVolumeAppearanceOverrides = {
  thresholdOffset?: number;
  opticalThicknessScale?: number;
  transitionRangeScale?: number;
};

export type WeatherVolumeAppearance = {
  color: number;
  threshold: number;
  transitionRange: number;
  sampleOpacity: number;
  scale: { x: number; y: number; z: number };
  verticalOffset: number;
};

const MINIMUM_RAY_STEPS = 1;

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function volumeColor(effect: WeatherRenderEffect): number {
  switch (effect) {
    case "convective":
      return 0x93a0b4;
    case "snow":
      return 0xe3f0f3;
    case "obscuration":
      return 0xb9c4c8;
    case "dust":
      return 0xb98753;
    default:
      return 0xc4d1d9;
  }
}

export function weatherCloudColor(effect: WeatherRenderEffect): number {
  return volumeColor(effect);
}

/**
 * Converts total optical thickness into a per-sample alpha. This keeps the
 * broad cloud opacity stable when adaptive quality changes the ray-step count.
 */
export function volumeSampleOpacity(
  opticalThickness: number,
  steps: number,
): number {
  const boundedThickness = clamp(opticalThickness, 0, 24);
  const boundedSteps = Math.max(MINIMUM_RAY_STEPS, Math.floor(steps));
  return 1 - Math.exp(-boundedThickness / boundedSteps);
}

export function resolveWeatherVolumeAppearance(
  effect: WeatherRenderEffect,
  intensity: number,
  steps: number,
  overrides: WeatherVolumeAppearanceOverrides = {},
): WeatherVolumeAppearance {
  const boundedIntensity = clamp(intensity, 0, 1);
  const dust = effect === "dust";
  const thresholdBase =
    effect === "convective"
      ? 0.2
      : effect === "obscuration"
        ? 0.28
        : dust
          ? 0.27
          : 0.25;
  const thresholdIntensityInfluence = dust ? 0.025 : 0.045;
  const threshold = clamp(
    thresholdBase -
      boundedIntensity * thresholdIntensityInfluence +
      (overrides.thresholdOffset ?? 0),
    0.08,
    0.62,
  );
  const transitionRange = clamp(
    (dust ? 0.15 : effect === "obscuration" ? 0.13 : 0.105) *
      (overrides.transitionRangeScale ?? 1),
    0.035,
    0.3,
  );
  const opticalThickness =
    (dust ? 4.6 : effect === "obscuration" ? 5.3 : 7.2) *
    (0.82 + boundedIntensity * 0.36) *
    clamp(overrides.opticalThicknessScale ?? 1, 0.25, 2.5);

  return {
    color: volumeColor(effect),
    threshold,
    transitionRange,
    sampleOpacity: volumeSampleOpacity(opticalThickness, steps),
    scale: dust
      ? { x: 158, y: 92, z: 62 }
      : effect === "convective"
        ? { x: 154, y: 104, z: 72 }
        : { x: 148, y: 90, z: 66 },
    verticalOffset: dust ? 0 : -20,
  };
}

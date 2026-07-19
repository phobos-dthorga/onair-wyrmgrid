import { deterministicWeatherUnit, hashWeatherText } from "./deterministic";

export type WeatherVolumeVariation = {
  densityThresholdOffset: number;
  meshRotationRadians: number;
  sampleRotation: { x: number; y: number; z: number };
  sampleOffset: { x: number; y: number; z: number };
  scale: { x: number; y: number; z: number };
};

export function weatherVolumeVariation(cellId: string): WeatherVolumeVariation {
  const seed = hashWeatherText(`${cellId}:volume-variation`);
  return {
    densityThresholdOffset: (deterministicWeatherUnit(seed, 0) - 0.5) * 0.055,
    meshRotationRadians: (deterministicWeatherUnit(seed, 1) - 0.5) * 0.4,
    sampleRotation: {
      x: (deterministicWeatherUnit(seed, 2) - 0.5) * Math.PI * 0.8,
      y: (deterministicWeatherUnit(seed, 3) - 0.5) * Math.PI * 0.8,
      z: (deterministicWeatherUnit(seed, 4) - 0.5) * Math.PI * 1.5,
    },
    sampleOffset: {
      x: (deterministicWeatherUnit(seed, 5) - 0.5) * 0.09,
      y: (deterministicWeatherUnit(seed, 6) - 0.5) * 0.07,
      z: (deterministicWeatherUnit(seed, 7) - 0.5) * 0.09,
    },
    scale: {
      x: 0.88 + deterministicWeatherUnit(seed, 8) * 0.24,
      y: 0.9 + deterministicWeatherUnit(seed, 9) * 0.2,
      z: 0.88 + deterministicWeatherUnit(seed, 10) * 0.24,
    },
  };
}

import { deterministicWeatherUnit, hashWeatherText } from "./deterministic";

export type WeatherVolumeVariation = {
  densityThresholdOffset: number;
  rotationRadians: number;
};

export function weatherVolumeVariation(cellId: string): WeatherVolumeVariation {
  const seed = hashWeatherText(`${cellId}:volume-variation`);
  return {
    densityThresholdOffset: (deterministicWeatherUnit(seed, 0) - 0.5) * 0.07,
    rotationRadians: deterministicWeatherUnit(seed, 1) * Math.PI * 2 - Math.PI,
  };
}

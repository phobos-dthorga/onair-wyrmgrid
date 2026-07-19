import { describe, expect, it } from "vitest";
import {
  resolveWeatherVolumeAppearance,
  volumeSampleOpacity,
} from "./volumeAppearance";

function accumulatedOpacity(sampleOpacity: number, samples: number): number {
  return 1 - (1 - sampleOpacity) ** samples;
}

describe("weather volume appearance", () => {
  it("keeps total optical thickness stable across quality step counts", () => {
    const enhanced = accumulatedOpacity(volumeSampleOpacity(7.2, 32), 32);
    const cinematic = accumulatedOpacity(volumeSampleOpacity(7.2, 64), 64);

    expect(enhanced).toBeCloseTo(cinematic, 10);
    expect(enhanced).toBeGreaterThan(0.99);
  });

  it("produces broad cloud proportions rather than a capsule", () => {
    const cloud = resolveWeatherVolumeAppearance("cloud", 0.7, 48);

    expect(cloud.scale.x / cloud.scale.y).toBeLessThan(1.7);
    expect(cloud.scale.z).toBeGreaterThan(cloud.scale.y * 0.65);
    expect(cloud.threshold).toBeLessThan(0.3);
    expect(cloud.sampleOpacity).toBeGreaterThan(0.1);
  });

  it("bounds developer calibration overrides", () => {
    const appearance = resolveWeatherVolumeAppearance("rain", 0.8, 48, {
      thresholdOffset: -10,
      opticalThicknessScale: 10,
      transitionRangeScale: 10,
    });

    expect(appearance.threshold).toBe(0.08);
    expect(appearance.transitionRange).toBe(0.3);
    expect(appearance.sampleOpacity).toBeLessThan(1);
  });
});

import { describe, expect, it } from "vitest";
import { weatherVolumeVariation } from "./volumeVariation";

describe("weather volume variation", () => {
  it("is stable for one cell and varies between cells", () => {
    const first = weatherVolumeVariation("weather-cell-a");

    expect(weatherVolumeVariation("weather-cell-a")).toEqual(first);
    expect(weatherVolumeVariation("weather-cell-b")).not.toEqual(first);
  });

  it("keeps density, orientation, offset, and scale changes bounded", () => {
    for (const cellId of ["a", "b", "c", "d", "e"]) {
      const variation = weatherVolumeVariation(cellId);
      expect(Math.abs(variation.densityThresholdOffset)).toBeLessThanOrEqual(
        0.0275,
      );
      expect(variation.meshRotationRadians).toBeGreaterThanOrEqual(-0.2);
      expect(variation.meshRotationRadians).toBeLessThanOrEqual(0.2);
      expect(Math.abs(variation.sampleOffset.x)).toBeLessThanOrEqual(0.045);
      expect(Math.abs(variation.sampleOffset.y)).toBeLessThanOrEqual(0.035);
      expect(Math.abs(variation.sampleOffset.z)).toBeLessThanOrEqual(0.045);
      expect(variation.scale.x).toBeGreaterThanOrEqual(0.88);
      expect(variation.scale.x).toBeLessThanOrEqual(1.12);
      expect(variation.scale.y).toBeGreaterThanOrEqual(0.9);
      expect(variation.scale.y).toBeLessThanOrEqual(1.1);
      expect(variation.scale.z).toBeGreaterThanOrEqual(0.88);
      expect(variation.scale.z).toBeLessThanOrEqual(1.12);
    }
  });
});

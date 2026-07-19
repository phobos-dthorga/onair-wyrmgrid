import { describe, expect, it } from "vitest";
import { weatherVolumeVariation } from "./volumeVariation";

describe("weather volume variation", () => {
  it("is stable for one cell and varies between cells", () => {
    const first = weatherVolumeVariation("weather-cell-a");

    expect(weatherVolumeVariation("weather-cell-a")).toEqual(first);
    expect(weatherVolumeVariation("weather-cell-b")).not.toEqual(first);
  });

  it("keeps density and rotation changes inside their visual bounds", () => {
    for (const cellId of ["a", "b", "c", "d", "e"]) {
      const variation = weatherVolumeVariation(cellId);
      expect(Math.abs(variation.densityThresholdOffset)).toBeLessThanOrEqual(
        0.035,
      );
      expect(variation.rotationRadians).toBeGreaterThanOrEqual(-Math.PI);
      expect(variation.rotationRadians).toBeLessThanOrEqual(Math.PI);
    }
  });
});

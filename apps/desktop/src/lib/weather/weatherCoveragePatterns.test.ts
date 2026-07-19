import { describe, expect, it } from "vitest";
import {
  createWeatherZonePatternImage,
  WEATHER_ZONE_KINDS,
  weatherZonePatternExpression,
  weatherZonePatternId,
} from "./weatherCoveragePatterns";

function alphaSignature(data: Uint8ClampedArray): string {
  return Array.from({ length: data.length / 4 }, (_, index) =>
    data[index * 4 + 3] > 0 ? "1" : "0",
  ).join("");
}

describe("weather support-zone patterns", () => {
  it("creates power-of-two seamless fills with a unique motif per zone", () => {
    const signatures = new Set<string>();

    for (const kind of WEATHER_ZONE_KINDS) {
      const image = createWeatherZonePatternImage(kind, "fill");
      expect(image.width).toBe(64);
      expect(image.height).toBe(64);
      expect(image.data).toHaveLength(64 * 64 * 4);
      const signature = alphaSignature(image.data);
      expect(signature).toContain("1");
      expect(signature).toContain("0");
      signatures.add(signature);
    }

    expect(signatures.size).toBe(WEATHER_ZONE_KINDS.length);
  });

  it("bounds patterned airport markers to a transparent circle", () => {
    for (const kind of WEATHER_ZONE_KINDS) {
      const image = createWeatherZonePatternImage(kind, "marker");
      const alphaAt = (x: number, y: number) =>
        image.data[(y * image.width + x) * 4 + 3];

      expect(alphaAt(0, 0)).toBe(0);
      expect(alphaAt(image.width - 1, 0)).toBe(0);
      expect(alphaAt(0, image.height - 1)).toBe(0);
      expect(alphaAt(image.width / 2, image.height / 2)).toBeGreaterThan(0);
    }
  });

  it("maps every zone kind to a stable image identifier", () => {
    const expression = JSON.stringify(
      weatherZonePatternExpression("condition", "fill"),
    );

    for (const kind of WEATHER_ZONE_KINDS) {
      expect(expression).toContain(weatherZonePatternId(kind, "fill"));
    }
  });
});

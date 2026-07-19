import { describe, expect, it } from "vitest";
import {
  daylightBandForSolarAltitude,
  daylightFeatureCollection,
  solarAltitudeDegrees,
  solarPositionAt,
} from "./daylight";

describe("Atlas daylight calculation", () => {
  it("places the subsolar latitude at the equinox and solstice", () => {
    const equinox = solarPositionAt(new Date("2024-03-20T12:00:00Z"));
    const solstice = solarPositionAt(new Date("2024-06-20T20:51:00Z"));

    expect(Math.abs(equinox.latitude)).toBeLessThan(0.6);
    expect(Math.abs(equinox.longitude)).toBeLessThan(5);
    expect(solstice.latitude).toBeGreaterThan(23);
    expect(solstice.latitude).toBeLessThan(24);
  });

  it("distinguishes the subsolar and anti-solar points", () => {
    const time = new Date("2026-07-19T01:30:00Z");
    const sun = solarPositionAt(time);
    const antiLongitude =
      sun.longitude >= 0 ? sun.longitude - 180 : sun.longitude + 180;

    expect(solarAltitudeDegrees(time, sun.longitude, sun.latitude)).toBeCloseTo(
      90,
      8,
    );
    expect(
      solarAltitudeDegrees(time, antiLongitude, -sun.latitude),
    ).toBeCloseTo(-90, 8);
  });

  it("uses the standard geometric twilight bands", () => {
    expect(daylightBandForSolarAltitude(0)).toBe("day");
    expect(daylightBandForSolarAltitude(-3)).toBe("civil_twilight");
    expect(daylightBandForSolarAltitude(-9)).toBe("nautical_twilight");
    expect(daylightBandForSolarAltitude(-15)).toBe("astronomical_twilight");
    expect(daylightBandForSolarAltitude(-25)).toBe("night");
  });

  it("builds seam-safe spherical shade cells and terminator segments", () => {
    const geometry = daylightFeatureCollection(
      new Date("2026-07-19T01:30:00Z"),
      36,
    );

    expect(geometry.features).toHaveLength(5);
    expect(
      geometry.features.filter(
        (feature) => feature.properties.kind === "terminator",
      ),
    ).toHaveLength(1);
    for (const feature of geometry.features) {
      const coordinates =
        feature.geometry.type === "MultiPolygon"
          ? feature.geometry.coordinates.flat(2)
          : feature.geometry.coordinates.flat();
      for (const [longitude, latitude] of coordinates) {
        expect(Number.isFinite(longitude)).toBe(true);
        expect(longitude).toBeGreaterThanOrEqual(-180);
        expect(longitude).toBeLessThanOrEqual(180);
        expect(latitude).toBeGreaterThanOrEqual(-90);
        expect(latitude).toBeLessThanOrEqual(90);
      }
    }
  });

  it("never emits a shade face that can wrap across the globe", () => {
    const geometry = daylightFeatureCollection(
      new Date("2026-07-19T02:00:00Z"),
      90,
    );
    const shadeFeatures = geometry.features.filter(
      (feature) => feature.geometry.type === "MultiPolygon",
    );

    for (const feature of shadeFeatures) {
      if (feature.geometry.type !== "MultiPolygon") continue;
      for (const polygon of feature.geometry.coordinates) {
        const longitudes = polygon[0].map(([longitude]) => longitude);
        expect(
          Math.max(...longitudes) - Math.min(...longitudes),
        ).toBeLessThanOrEqual(4);
      }
    }
  });

  it("keeps odd-resolution geometry inside WGS84 and splits terminator seams", () => {
    const geometry = daylightFeatureCollection(
      new Date("2026-07-19T02:00:00Z"),
      91,
    );
    const terminator = geometry.features.find(
      (feature) => feature.geometry.type === "MultiLineString",
    );

    expect(terminator?.geometry.type).toBe("MultiLineString");
    if (terminator?.geometry.type !== "MultiLineString") return;
    for (const line of terminator.geometry.coordinates) {
      const longitudes = line.map(([longitude]) => longitude);
      const latitudes = line.map(([, latitude]) => latitude);
      expect(
        Math.max(...longitudes) - Math.min(...longitudes),
      ).toBeLessThanOrEqual(180);
      expect(Math.min(...latitudes)).toBeGreaterThanOrEqual(-90);
      expect(Math.max(...latitudes)).toBeLessThanOrEqual(90);
    }
  });

  it("rejects invalid times and unbounded geometry requests", () => {
    expect(() => solarPositionAt(new Date(Number.NaN))).toThrow(RangeError);
    expect(() =>
      daylightFeatureCollection(new Date("2026-07-19T00:00:00Z"), 8),
    ).toThrow(RangeError);
  });
});

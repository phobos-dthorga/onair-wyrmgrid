import { describe, expect, it } from "vitest";
import type {
  GlobalWeatherGridPoint,
  PublishedPluginWeatherLayer,
} from "$lib/forge/types";
import type { FlightWeatherMapView } from "$lib/weather/types";
import {
  pluginRadarCoverageFeatures,
  pluginWeatherGridCoverageFeatures,
  weatherSupportZoneCount,
} from "./weatherCoverage";

const provenance = {
  kind: "external_calculation" as const,
  provider: "example.test",
  retrieved_at: "2026-07-19T00:00:00Z",
  transformation_version: 1,
  freshness: "current" as const,
};

function gridLayer(
  points: GlobalWeatherGridPoint[],
): PublishedPluginWeatherLayer {
  return {
    plugin_id: "org.example.weather",
    plugin_name: "Example Weather",
    layer: {
      schema_version: 1,
      id: "regular-grid",
      title: "Regular grid",
      data: { kind: "grid", points },
      provenance,
    },
  };
}

function point(
  id: string,
  latitude: number,
  longitude: number,
  condition: GlobalWeatherGridPoint["condition"] = "rain",
): GlobalWeatherGridPoint {
  return { id, location: { latitude, longitude }, condition };
}

describe("weather support-zone geometry", () => {
  it("builds compact circular footprints only for a complete regular grid", () => {
    const layer = gridLayer([
      point("nw", 10, -10, "cloud"),
      point("ne", 10, 10, "rain"),
      point("sw", -10, -10, "snow"),
      point("se", -10, 10, "convective"),
    ]);
    const coverage = pluginWeatherGridCoverageFeatures([layer]);

    expect(coverage.features).toHaveLength(4);
    expect(coverage.features[0].properties).toMatchObject({
      coverage_kind: "model_sample_footprint",
      support_radius_nm: 180,
      extent_basis: "sample_support",
    });
    const northwest = coverage.features.find((feature) =>
      feature.properties.id.includes(":nw:"),
    );
    const ring = northwest?.geometry.coordinates[0] ?? [];
    expect(ring).toHaveLength(49);
    expect(ring[0]).toEqual(ring.at(-1));
    expect(ring[0][0]).toBeCloseTo(-10, 8);
    expect(ring[0][1]).toBeCloseTo(12.9979, 3);
    expect(ring[12][0]).toBeCloseTo(-6.956, 3);
    expect(ring[12][1]).toBeCloseTo(9.986, 3);
  });

  it("uses only an explicit provider radius for variable weather-pattern size", () => {
    const northwest = point("nw", 2, -2);
    northwest.provider_extent_radius_nm = 42;
    const layer = gridLayer([
      northwest,
      point("ne", 2, 2),
      point("sw", -2, -2),
      point("se", -2, 2),
    ]);
    const coverage = pluginWeatherGridCoverageFeatures([layer]);
    const sourced = coverage.features.find((feature) =>
      feature.properties.id.includes(":nw:"),
    );
    const unsourced = coverage.features.find((feature) =>
      feature.properties.id.includes(":ne:"),
    );

    expect(sourced?.properties).toMatchObject({
      support_radius_nm: 42,
      extent_basis: "provider_reported",
    });
    expect(unsourced?.properties.extent_basis).toBe("sample_support");
    expect(unsourced?.properties.support_radius_nm).toBeCloseTo(120.01, 1);
  });

  it("accepts an isolated extent only when the provider explicitly supplies it", () => {
    const sourced = point("source-defined", 25, 130);
    sourced.provider_extent_radius_nm = 55;
    const unsourced = point("point-only", 35, 140);

    const coverage = pluginWeatherGridCoverageFeatures([
      gridLayer([sourced, unsourced]),
    ]);

    expect(coverage.features).toHaveLength(1);
    expect(coverage.features[0].properties).toMatchObject({
      support_radius_nm: 55,
      extent_basis: "provider_reported",
    });
  });

  it("omits a provider circle that cannot be represented safely across the antimeridian", () => {
    const sourced = point("crossing", 0, 179);
    sourced.provider_extent_radius_nm = 120;

    expect(
      pluginWeatherGridCoverageFeatures([gridLayer([sourced])]).features,
    ).toEqual([]);
  });

  it("limits dense-grid footprints to half the nearest sample spacing", () => {
    const layer = gridLayer([
      point("nw", 2, -2),
      point("ne", 2, 2),
      point("sw", -2, -2),
      point("se", -2, 2),
    ]);
    const coverage = pluginWeatherGridCoverageFeatures([layer]);
    const northwest = coverage.features.find((feature) =>
      feature.properties.id.includes(":nw:"),
    );

    expect(northwest?.properties.support_radius_nm).toBeCloseTo(120.01, 1);
    const ring = northwest?.geometry.coordinates[0] ?? [];
    expect(ring[0][1]).toBeCloseTo(4, 2);
    expect(ring[12][0]).toBeCloseTo(0, 2);
  });

  it("refuses incomplete, duplicate, and irregular point sets", () => {
    const incomplete = gridLayer([
      point("a", -10, -10),
      point("b", -10, 10),
      point("c", 10, -10),
    ]);
    const duplicate = gridLayer([
      point("a", -10, -10),
      point("b", -10, 10),
      point("c", 10, -10),
      point("d", 10, -10),
    ]);
    const irregular = gridLayer([
      point("a", -10, -170),
      point("b", -10, 170),
      point("c", 10, -170),
      point("d", 10, 170),
    ]);

    expect(pluginWeatherGridCoverageFeatures([incomplete]).features).toEqual(
      [],
    );
    expect(pluginWeatherGridCoverageFeatures([duplicate]).features).toEqual([]);
    expect(pluginWeatherGridCoverageFeatures([irregular]).features).toEqual([]);
  });

  it("keeps the fixed host grid's circular support zones inside the antimeridian", () => {
    const layer = gridLayer([
      point("a", -10, -165),
      point("b", -10, -135),
      point("c", 10, -165),
      point("d", 10, -135),
    ]);
    const coverage = pluginWeatherGridCoverageFeatures([layer]);
    const western = coverage.features.find((feature) =>
      feature.properties.id.includes(":a:"),
    );

    const longitudes = western?.geometry.coordinates[0].map(
      (coordinate) => coordinate[0],
    );
    expect(Math.min(...(longitudes ?? []))).toBeGreaterThan(-180);
    expect(Math.max(...(longitudes ?? []))).toBeLessThan(180);
  });

  it("outlines exactly the validated received RADAR tile footprint", () => {
    const radar: PublishedPluginWeatherLayer = {
      plugin_id: "org.example.radar",
      plugin_name: "Example RADAR",
      layer: {
        schema_version: 1,
        id: "radar",
        title: "RADAR",
        data: {
          kind: "raster_tiles",
          frame_time: "2026-07-19T00:00:00Z",
          tiles: [{ zoom: 1, x: 0, y: 0, png_base64: "iVBORw0KGgo=" }],
        },
        provenance: { ...provenance, kind: "external_fact" },
      },
    };
    const coverage = pluginRadarCoverageFeatures([radar]);

    expect(coverage.features).toHaveLength(1);
    expect(coverage.features[0].properties).toMatchObject({
      coverage_kind: "radar_tile",
      support_radius_nm: null,
      extent_basis: null,
      condition: "radar",
      frame_time: "2026-07-19T00:00:00Z",
    });
    const ring = coverage.features[0].geometry.coordinates[0];
    expect(ring[0]).toEqual(ring.at(-1));
    expect(ring[0][0]).toBe(-180);
    expect(ring[0][1]).toBeCloseTo(85.05112878, 8);
    expect(ring[2]).toEqual([0, 0]);
  });

  it("counts only support zones that the renderer can actually show", () => {
    const weather: FlightWeatherMapView = {
      schema_version: 1,
      plan_id: "plan-1",
      weather_snapshot_id: "weather-1",
      stations: [
        {
          id: "weather:origin:yssy",
          role: "origin",
          station_icao: "YSSY",
          location: { latitude: -33.9461, longitude: 151.1772 },
          metar: {
            value: {
              observed_at: "2026-07-19T00:00:00Z",
              raw_text: "YSSY 190000Z -SHRA",
              present_weather: "-SHRA",
            },
            provenance: { ...provenance, kind: "external_fact" },
          },
        },
      ],
    };
    const layer = gridLayer([
      point("nw", 10, -10, "clear"),
      point("ne", 10, 10, "unknown"),
      point("sw", -10, -10, "rain"),
      point("se", -10, 10, "snow"),
    ]);

    expect(weatherSupportZoneCount(weather, [layer])).toBe(3);
  });
});

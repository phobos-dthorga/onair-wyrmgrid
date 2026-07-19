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
  it("builds midpoint cells only for a complete regular grid", () => {
    const layer = gridLayer([
      point("nw", 10, -10, "cloud"),
      point("ne", 10, 10, "rain"),
      point("sw", -10, -10, "snow"),
      point("se", -10, 10, "convective"),
    ]);
    const coverage = pluginWeatherGridCoverageFeatures([layer]);

    expect(coverage.features).toHaveLength(4);
    expect(coverage.features[0].properties.coverage_kind).toBe(
      "model_sample_cell",
    );
    const northwest = coverage.features.find((feature) =>
      feature.properties.id.includes(":nw:"),
    );
    expect(northwest?.geometry.coordinates[0]).toEqual([
      [-20, 20],
      [0, 20],
      [0, 0],
      [-20, 0],
      [-20, 20],
    ]);
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

  it("keeps a regular global grid bounded at the antimeridian", () => {
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

    expect(western?.geometry.coordinates[0][0][0]).toBe(-180);
    expect(western?.geometry.coordinates[0][1][0]).toBe(-150);
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

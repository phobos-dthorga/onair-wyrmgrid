import { describe, expect, it } from "vitest";
import type { PublishedPluginWeatherLayer } from "$lib/forge/types";
import {
  pluginRadarFrames,
  pluginWeatherGridFeatures,
  pluginWeatherItemCount,
  rasterTileCoordinates,
} from "./pluginWeather";

const provenance = {
  kind: "external_calculation" as const,
  provider: "example.test",
  retrieved_at: "2026-07-17T12:00:00Z",
  transformation_version: 1,
  freshness: "current" as const,
};

describe("plugin weather presentation", () => {
  it("projects provider-neutral grid samples without inventing missing values", () => {
    const layers: PublishedPluginWeatherLayer[] = [
      {
        plugin_id: "org.example.weather",
        plugin_name: "Example Weather",
        layer: {
          schema_version: 1,
          id: "global-grid",
          title: "Global grid",
          data: {
            kind: "grid",
            points: [
              {
                id: "point-1",
                location: { latitude: -33.86, longitude: 151.2 },
                condition: "rain",
                precipitation_mm: 2.5,
              },
            ],
          },
          provenance,
        },
      },
    ];

    expect(pluginWeatherGridFeatures(layers).features[0]).toMatchObject({
      geometry: { coordinates: [151.2, -33.86] },
      properties: {
        condition: "rain",
        precipitation_mm: 2.5,
        temperature_c: null,
      },
    });
    expect(pluginWeatherItemCount(layers)).toBe(1);
  });

  it("converts validated XYZ tiles into host-owned image corners", () => {
    const corners = rasterTileCoordinates({ zoom: 1, x: 0, y: 0 });
    expect(corners[0][0]).toBe(-180);
    expect(corners[0][1]).toBeCloseTo(85.05112878, 8);
    expect(corners[1][0]).toBe(0);
    expect(corners[1][1]).toBeCloseTo(85.05112878, 8);
    expect(corners[2]).toEqual([0, 0]);
    expect(corners[3]).toEqual([-180, 0]);
    const layers: PublishedPluginWeatherLayer[] = [
      {
        plugin_id: "org.example.radar",
        plugin_name: "Example Radar",
        layer: {
          schema_version: 1,
          id: "radar",
          title: "Radar",
          data: {
            kind: "raster_tiles",
            frame_time: "2026-07-17T12:00:00Z",
            tiles: [{ zoom: 1, x: 0, y: 0, png_base64: "iVBORw0KGgo=" }],
          },
          provenance: { ...provenance, kind: "external_fact" },
        },
      },
    ];
    expect(pluginRadarFrames(layers)[0].url).toBe(
      "data:image/png;base64,iVBORw0KGgo=",
    );
  });
});

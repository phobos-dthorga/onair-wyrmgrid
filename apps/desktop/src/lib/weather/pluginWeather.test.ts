import { describe, expect, it } from "vitest";
import type { PublishedPluginWeatherLayer } from "$lib/forge/types";
import {
  displayedGlobalWeatherGridPoints,
  pluginRadarTimelines,
  pluginWeatherGridFeatures,
  pluginWeatherItemCount,
  longestRadarTimeline,
  rasterTileCoordinates,
  selectedRadarFrames,
  weatherLayersForTemporalMode,
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

  it("draws only the forecast horizon nearest retrieval at each location", () => {
    const layer: PublishedPluginWeatherLayer["layer"] = {
      schema_version: 1,
      id: "temporal-grid",
      title: "Temporal grid",
      data: {
        kind: "grid",
        points: [
          {
            id: "sydney-h06",
            location: { latitude: -33.86, longitude: 151.2 },
            valid_at: "2026-07-17T18:00:00Z",
            condition: "rain",
          },
          {
            id: "sydney-h00",
            location: { latitude: -33.86, longitude: 151.2 },
            valid_at: "2026-07-17T12:00:00Z",
            condition: "clear",
          },
          {
            id: "legacy-melbourne",
            location: { latitude: -37.81, longitude: 144.96 },
            condition: "cloud",
          },
        ],
      },
      provenance,
    };

    expect(
      displayedGlobalWeatherGridPoints(layer).map((point) => point.id),
    ).toEqual(["sydney-h00", "legacy-melbourne"]);
    const published = [
      {
        plugin_id: "org.example.weather",
        plugin_name: "Example Weather",
        layer,
      },
    ];
    expect(pluginWeatherGridFeatures(published).features).toHaveLength(2);
    expect(pluginWeatherItemCount(published)).toBe(2);
  });

  it("keeps historical model layers separate from live weather", () => {
    const live: PublishedPluginWeatherLayer = {
      plugin_id: "org.example.weather",
      plugin_name: "Example Weather",
      layer: {
        schema_version: 1,
        id: "live",
        title: "Live",
        data: {
          kind: "grid",
          points: [
            {
              id: "live-1",
              location: { latitude: 0, longitude: 0 },
              condition: "clear",
            },
          ],
        },
        provenance,
      },
    };
    const historical: PublishedPluginWeatherLayer = {
      ...live,
      layer: {
        ...live.layer,
        id: "historical",
        title: "Historical",
        time_scope: {
          kind: "historical_model",
          target_at: "2026-07-12T12:00:00Z",
          starts_at: "2026-07-12T08:00:00Z",
          ends_at: "2026-07-12T16:00:00Z",
        },
      },
    };
    const archived = {
      ...historical,
      layer: {
        ...historical.layer,
        id: "archived",
        time_scope: {
          ...historical.layer.time_scope!,
          kind: "archived_forecast" as const,
        },
      },
    };

    expect(
      weatherLayersForTemporalMode([live, historical, archived], "live"),
    ).toEqual([live]);
    expect(
      weatherLayersForTemporalMode([live, historical, archived], "historical"),
    ).toEqual([historical, archived]);
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
    const timeline = pluginRadarTimelines(layers)[0];
    expect(timeline.frames[0].tiles[0].url).toBe(
      "data:image/png;base64,iVBORw0KGgo=",
    );
  });

  it("orders factual RADAR frames and exposes optional no-data masks", () => {
    const layers: PublishedPluginWeatherLayer[] = ["12:10:00", "12:00:00"].map(
      (time, index) => ({
        plugin_id: "org.example.radar",
        plugin_name: "Example Radar",
        layer: {
          schema_version: 1,
          id: "radar",
          title: "Radar",
          data: {
            kind: "raster_tiles" as const,
            frame_time: `2026-07-17T${time}Z`,
            tiles: [
              {
                zoom: 1,
                x: 0,
                y: 0,
                png_base64: `radar-${index}`,
                coverage_png_base64: `coverage-${index}`,
              },
            ],
          },
          provenance: { ...provenance, kind: "external_fact" as const },
        },
      }),
    );

    const timeline = pluginRadarTimelines(layers)[0];
    expect(timeline.frames.map((frame) => frame.frame_time)).toEqual([
      "2026-07-17T12:00:00Z",
      "2026-07-17T12:10:00Z",
    ]);
    expect(timeline.frames[0].tiles[0].coverage_url).toBe(
      "data:image/png;base64,coverage-1",
    );
    expect(selectedRadarFrames([timeline], 0, false)[0].frame_time).toBe(
      "2026-07-17T12:00:00Z",
    );
    expect(selectedRadarFrames([timeline], 0, true)[0].frame_time).toBe(
      "2026-07-17T12:10:00Z",
    );
    expect(
      longestRadarTimeline([
        { ...timeline, id: "static", frames: [timeline.frames[0]] },
        timeline,
      ])?.id,
    ).toBe(timeline.id);
  });
});

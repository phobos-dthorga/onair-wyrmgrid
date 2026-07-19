import { describe, expect, it } from "vitest";
import type { PublishedPluginWeatherLayer } from "$lib/forge/types";
import type { FlightWeatherMapView } from "$lib/weather/types";
import { buildWeatherRenderScene } from "./weatherRenderScene";

const airportWeather: FlightWeatherMapView = {
  schema_version: 1,
  plan_id: "plan-1",
  weather_snapshot_id: "weather-1",
  stations: [
    {
      id: "weather:origin:yssy",
      role: "origin",
      station_icao: "YSSY",
      location: { longitude: 151.177, latitude: -33.946 },
      metar: {
        provenance: {
          kind: "external_fact",
          provider: "aviationweather.gov",
          retrieved_at: "2026-07-18T00:00:00Z",
          transformation_version: 1,
          freshness: "current",
        },
        value: {
          observed_at: "2026-07-18T00:00:00Z",
          raw_text: "YSSY 180000Z 18020KT +TSRA",
          present_weather: "+TSRA",
          wind_direction: { kind: "degrees", value: 180 },
          wind_speed_kt: 20,
        },
      },
    },
    {
      id: "weather:destination:nzaa",
      role: "destination",
      station_icao: "NZAA",
      location: { longitude: 174.785, latitude: -37.008 },
    },
  ],
};

const pluginWeather: PublishedPluginWeatherLayer[] = [
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
            id: "rain",
            location: { longitude: 10, latitude: 20 },
            condition: "rain",
            precipitation_mm: 40,
            cloud_cover_percent: 75,
            wind_direction_degrees: 270,
            wind_speed_kt: 30,
          },
          {
            id: "unknown",
            location: { longitude: 11, latitude: 21 },
            condition: "unknown",
          },
        ],
      },
      provenance: {
        kind: "external_calculation",
        provider: "example.test",
        retrieved_at: "2026-07-18T00:00:00Z",
        transformation_version: 1,
        freshness: "current",
      },
    },
  },
];

describe("Three.js weather render scene", () => {
  it("projects only explicit airport and plugin phenomena", () => {
    const scene = buildWeatherRenderScene(airportWeather, pluginWeather);

    expect(scene.cells).toHaveLength(2);
    expect(scene.cells[0]).toMatchObject({
      id: "weather:origin:yssy",
      source: "airport",
      effect: "convective",
      intensity: 1,
      windBearing: 0,
    });
    expect(scene.cells[1]).toMatchObject({
      id: "org.example.weather:global-grid:rain",
      source: "plugin_grid",
      effect: "rain",
      intensity: 1,
      windBearing: 90,
    });
    expect(scene.signature).toContain("weather:origin:yssy");
    expect(scene.signature).not.toContain("unknown");
  });

  it("does not invent cells for absent or clear weather", () => {
    expect(buildWeatherRenderScene(undefined, []).cells).toEqual([]);
  });
});

import { describe, expect, it } from "vitest";
import type { FlightWeatherMapView } from "$lib/weather/types";
import {
  weatherMapSignature,
  weatherPointCoordinates,
  weatherStationFeatures,
} from "./atlasWeather";

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
    },
    {
      id: "weather:destination:nzaa",
      role: "destination",
      station_icao: "NZAA",
      metar: {
        value: {
          observed_at: "2026-07-16T00:00:00Z",
          raw_text: "NZAA 160000Z 00000KT CAVOK 12/08 Q1018",
          flight_category: "vfr",
        },
        provenance: {
          kind: "external_fact",
          provider: "aviationweather.gov",
          retrieved_at: "2026-07-16T00:03:00Z",
          transformation_version: 1,
          freshness: "current",
        },
      },
    },
  ],
};

describe("Atlas airport-weather projection", () => {
  it("plots sourced coordinates with unknown weather instead of inventing clear skies", () => {
    const feature = weatherStationFeatures(weather).features[0];
    expect(feature.properties).toMatchObject({
      station_icao: "YSSY",
      category: "unknown",
      has_metar: false,
      has_taf: false,
    });
    expect(feature.geometry.coordinates).toEqual([151.1772, -33.9461]);
  });

  it("keeps an unlocated report in the evidence signature without plotting it", () => {
    expect(weatherStationFeatures(weather).features).toHaveLength(1);
    expect(weatherMapSignature(weather)).toContain("weather:destination:nzaa");
    expect(
      weatherPointCoordinates(weather, "weather:destination:nzaa"),
    ).toBeUndefined();
  });
});

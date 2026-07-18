import { describe, expect, it } from "vitest";
import type { FlightWeatherMapView } from "$lib/weather/types";
import {
  weatherEffect,
  weatherMapSignature,
  weatherPointCoordinates,
  weatherStationFeatures,
  weatherWindFeatures,
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

  it("derives bounded visual effects only from explicit present-weather codes", () => {
    expect(weatherEffect("-SHRA BR")).toBe("rain");
    expect(weatherEffect("TSRA")).toBe("convective");
    expect(weatherEffect("BKN020")).toBe("none");
    expect(weatherEffect(undefined)).toBe("none");
  });

  it("projects a sourced wind vector in the direction the air is moving", () => {
    const windWeather: FlightWeatherMapView = {
      ...weather,
      stations: [
        {
          id: "weather:origin:yssy",
          role: "origin",
          station_icao: "YSSY",
          location: { latitude: -33.9461, longitude: 151.1772 },
          metar: {
            value: {
              observed_at: "2026-07-16T00:00:00Z",
              raw_text: "YSSY 160000Z 32020G30KT 9999 -SHRA SCT020",
              flight_category: "mvfr",
              wind_direction: { kind: "degrees", value: 320 },
              wind_speed_kt: 20,
              wind_gust_kt: 30,
              present_weather: "-SHRA",
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

    const station = weatherStationFeatures(windWeather).features[0];
    expect(station.properties).toMatchObject({
      severity: 0.42,
      effect: "rain",
      wind_speed_kt: 20,
      wind_gust_kt: 30,
    });

    const wind = weatherWindFeatures(windWeather);
    expect(wind.features).toHaveLength(2);
    const path = wind.features.find(
      ({ properties }) => properties.feature_type === "wind_path",
    );
    expect(path?.geometry.type).toBe("LineString");
    if (path?.geometry.type !== "LineString") return;
    expect(path.properties.bearing).toBe(140);
    expect(path.geometry.coordinates[1][0]).toBeGreaterThan(151.1772);
    expect(path.geometry.coordinates[1][1]).toBeLessThan(-33.9461);
  });

  it("does not invent a wind vector for calm or variable reports", () => {
    const variableWeather: FlightWeatherMapView = {
      ...weather,
      stations: [
        {
          ...weather.stations[0],
          metar: {
            value: {
              observed_at: "2026-07-16T00:00:00Z",
              raw_text: "YSSY 160000Z VRB00KT CAVOK",
              wind_direction: { kind: "variable" },
              wind_speed_kt: 0,
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
    expect(weatherWindFeatures(variableWeather).features).toEqual([]);
  });
});

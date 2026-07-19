import type { PublishedPluginWeatherLayer } from "$lib/forge/types";
import { weatherStationFeatures } from "$lib/weather/atlasWeather";
import { pluginWeatherGridFeatures } from "$lib/weather/pluginWeather";
import type { FlightWeatherMapView } from "$lib/weather/types";

export type WeatherRenderEffect =
  "cloud" | "rain" | "snow" | "convective" | "dust" | "obscuration";

export type WeatherRenderCell = {
  id: string;
  source: "airport" | "plugin_grid";
  longitude: number;
  latitude: number;
  effect: WeatherRenderEffect;
  intensity: number;
  windBearing: number;
  windSpeedKt: number;
};

export type WeatherRenderScene = {
  signature: string;
  cells: WeatherRenderCell[];
};

const MAX_VISUAL_PRECIPITATION_MM = 8;

function clampUnit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function pluginCellIntensity(
  effect: WeatherRenderEffect,
  precipitationMm: number | null,
  cloudCoverPercent: number | null,
): number {
  const precipitation = clampUnit(
    (precipitationMm ?? 0) / MAX_VISUAL_PRECIPITATION_MM,
  );
  const cloudCover = clampUnit((cloudCoverPercent ?? 0) / 100);
  switch (effect) {
    case "convective":
      return Math.max(0.78, precipitation, cloudCover);
    case "rain":
    case "snow":
      return Math.max(0.35, precipitation, cloudCover * 0.7);
    case "cloud":
      return Math.max(0.25, cloudCover);
    case "obscuration":
      return Math.max(0.45, cloudCover);
    case "dust":
      return 0.6;
  }
}

function pluginEffect(condition: string): WeatherRenderEffect | undefined {
  switch (condition) {
    case "cloud":
    case "rain":
    case "snow":
    case "convective":
    case "obscuration":
      return condition;
    default:
      return undefined;
  }
}

export function buildWeatherRenderScene(
  weather: FlightWeatherMapView | undefined,
  pluginLayers: readonly PublishedPluginWeatherLayer[],
): WeatherRenderScene {
  const airportCells: WeatherRenderCell[] = weatherStationFeatures(weather)
    .features.filter(({ properties }) => properties.effect !== "none")
    .map(({ geometry, properties }) => ({
      id: properties.id,
      source: "airport" as const,
      longitude: geometry.coordinates[0],
      latitude: geometry.coordinates[1],
      effect: properties.effect as WeatherRenderEffect,
      intensity: properties.intensity,
      windBearing: properties.wind_bearing,
      windSpeedKt: properties.wind_speed_kt,
    }));

  const pluginCells: WeatherRenderCell[] = pluginWeatherGridFeatures(
    pluginLayers,
  ).features.flatMap(({ geometry, properties }) => {
    const effect = pluginEffect(properties.condition);
    if (!effect) return [];
    return [
      {
        id: properties.id,
        source: "plugin_grid" as const,
        longitude: geometry.coordinates[0],
        latitude: geometry.coordinates[1],
        effect,
        intensity: pluginCellIntensity(
          effect,
          properties.precipitation_mm,
          properties.cloud_cover_percent,
        ),
        windBearing:
          properties.wind_direction_degrees === null
            ? 0
            : (properties.wind_direction_degrees + 180) % 360,
        windSpeedKt: properties.wind_speed_kt ?? 0,
      },
    ];
  });

  const cells = [...airportCells, ...pluginCells];
  return {
    signature: cells
      .map(
        (cell) =>
          `${cell.id}:${cell.longitude}:${cell.latitude}:${cell.effect}:${cell.intensity}:${cell.windBearing}:${cell.windSpeedKt}`,
      )
      .join("|"),
    cells,
  };
}

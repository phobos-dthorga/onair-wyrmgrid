import type {
  GlobalWeatherCondition,
  GlobalWeatherGridPoint,
  PublishedPluginWeatherLayer,
} from "$lib/forge/types";
import { weatherStationFeatures } from "./atlasWeather";
import { pluginRadarTimelines } from "./pluginWeather";
import type { FlightWeatherMapView } from "./types";

type Coordinate = [number, number];

export type WeatherCoverageFeatureCollection = {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    geometry: { type: "Polygon"; coordinates: Coordinate[][] };
    properties: {
      id: string;
      plugin_id: string;
      layer_title: string;
      condition: GlobalWeatherCondition | "radar";
      coverage_kind: "model_sample_cell" | "radar_tile";
      frame_time: string | null;
    };
  }>;
};

const COORDINATE_PRECISION = 6;
const MAXIMUM_REGULAR_SPACING_RATIO = 1.35;
const MAXIMUM_LONGITUDE_SPACING = 90;
const MAXIMUM_MODEL_SUPPORT_SPAN_DEGREES = 6;

function coordinateKey(value: number): string {
  return value.toFixed(COORDINATE_PRECISION);
}

function uniqueSorted(values: readonly number[]): number[] {
  return [
    ...new Map(values.map((value) => [coordinateKey(value), value])).values(),
  ].sort((left, right) => left - right);
}

function hasRegularSpacing(
  values: readonly number[],
  maximumGap: number,
): boolean {
  if (values.length < 2) return false;
  const gaps = values.slice(1).map((value, index) => value - values[index]);
  const minimum = Math.min(...gaps);
  const maximum = Math.max(...gaps);
  return (
    minimum > 0 &&
    maximum <= maximumGap &&
    maximum / minimum <= MAXIMUM_REGULAR_SPACING_RATIO
  );
}

function coordinateBounds(
  values: readonly number[],
  index: number,
  minimum: number,
  maximum: number,
  maximumSpan: number,
): [number, number] {
  const value = values[index];
  const midpointLower =
    index === 0
      ? value - (values[index + 1] - value) / 2
      : (values[index - 1] + value) / 2;
  const midpointUpper =
    index === values.length - 1
      ? value + (value - values[index - 1]) / 2
      : (value + values[index + 1]) / 2;
  const maximumHalfSpan = maximumSpan / 2;
  return [
    Math.max(minimum, midpointLower, value - maximumHalfSpan),
    Math.min(maximum, midpointUpper, value + maximumHalfSpan),
  ];
}

function completeRegularGrid(
  points: readonly GlobalWeatherGridPoint[],
): { latitudes: number[]; longitudes: number[] } | undefined {
  const latitudes = uniqueSorted(
    points.map((point) => point.location.latitude),
  );
  const longitudes = uniqueSorted(
    points.map((point) => point.location.longitude),
  );
  if (
    !hasRegularSpacing(latitudes, 60) ||
    !hasRegularSpacing(longitudes, MAXIMUM_LONGITUDE_SPACING) ||
    points.length !== latitudes.length * longitudes.length
  ) {
    return undefined;
  }
  const combinations = new Set(
    points.map(
      (point) =>
        `${coordinateKey(point.location.latitude)}:${coordinateKey(point.location.longitude)}`,
    ),
  );
  if (combinations.size !== points.length) return undefined;
  for (const latitude of latitudes) {
    for (const longitude of longitudes) {
      if (
        !combinations.has(
          `${coordinateKey(latitude)}:${coordinateKey(longitude)}`,
        )
      ) {
        return undefined;
      }
    }
  }
  return { latitudes, longitudes };
}

function gridCoverageFeatures(
  published: PublishedPluginWeatherLayer,
): WeatherCoverageFeatureCollection["features"] {
  const data = published.layer.data;
  if (data.kind !== "grid") return [];
  const grid = completeRegularGrid(data.points);
  if (!grid) return [];
  const latitudeIndexes = new Map(
    grid.latitudes.map((value, index) => [coordinateKey(value), index]),
  );
  const longitudeIndexes = new Map(
    grid.longitudes.map((value, index) => [coordinateKey(value), index]),
  );
  return data.points.map((point) => {
    const latitudeIndex = latitudeIndexes.get(
      coordinateKey(point.location.latitude),
    );
    const longitudeIndex = longitudeIndexes.get(
      coordinateKey(point.location.longitude),
    );
    if (latitudeIndex === undefined || longitudeIndex === undefined) {
      throw new Error("Validated regular-grid coordinates were not indexed.");
    }
    const [south, north] = coordinateBounds(
      grid.latitudes,
      latitudeIndex,
      -90,
      90,
      MAXIMUM_MODEL_SUPPORT_SPAN_DEGREES,
    );
    const [west, east] = coordinateBounds(
      grid.longitudes,
      longitudeIndex,
      -180,
      180,
      MAXIMUM_MODEL_SUPPORT_SPAN_DEGREES,
    );
    return {
      type: "Feature" as const,
      geometry: {
        type: "Polygon" as const,
        coordinates: [
          [
            [west, north],
            [east, north],
            [east, south],
            [west, south],
            [west, north],
          ],
        ] as Coordinate[][],
      },
      properties: {
        id: `${published.plugin_id}:${published.layer.id}:${point.id}:support`,
        plugin_id: published.plugin_id,
        layer_title: published.layer.title,
        condition: point.condition,
        coverage_kind: "model_sample_cell" as const,
        frame_time: null,
      },
    };
  });
}

export function pluginWeatherGridCoverageFeatures(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): WeatherCoverageFeatureCollection {
  return {
    type: "FeatureCollection",
    features: publishedLayers.flatMap(gridCoverageFeatures),
  };
}

export function pluginRadarCoverageFeatures(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): WeatherCoverageFeatureCollection {
  return {
    type: "FeatureCollection",
    features: pluginRadarTimelines(publishedLayers).flatMap((timeline) => {
      const frame = timeline.frames.at(-1);
      if (!frame) return [];
      return frame.tiles.map((tile) => ({
        type: "Feature" as const,
        geometry: {
          type: "Polygon" as const,
          coordinates: [[...tile.coordinates, tile.coordinates[0]]],
        },
        properties: {
          id: `${tile.id}:footprint`,
          plugin_id: timeline.plugin_id,
          layer_title: timeline.layer_title,
          condition: "radar" as const,
          coverage_kind: "radar_tile" as const,
          frame_time: frame.frame_time,
        },
      }));
    }),
  };
}

export function weatherSupportZoneCount(
  weather: FlightWeatherMapView | undefined,
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): number {
  const airportZones = weatherStationFeatures(weather).features.filter(
    (feature) => feature.properties.effect !== "none",
  ).length;
  const modelZones = pluginWeatherGridCoverageFeatures(
    publishedLayers,
  ).features.filter(
    (feature) =>
      feature.properties.condition !== "clear" &&
      feature.properties.condition !== "unknown",
  ).length;
  return (
    airportZones +
    modelZones +
    pluginRadarCoverageFeatures(publishedLayers).features.length
  );
}

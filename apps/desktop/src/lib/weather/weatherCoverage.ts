import type {
  GlobalWeatherCondition,
  GlobalWeatherGridPoint,
  PublishedPluginWeatherLayer,
} from "$lib/forge/types";
import { weatherStationFeatures } from "./atlasWeather";
import {
  displayedGlobalWeatherGridPoints,
  pluginRadarTimelines,
} from "./pluginWeather";
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
      coverage_kind: "model_sample_footprint" | "radar_tile";
      support_radius_nm: number | null;
      extent_basis: "provider_reported" | "sample_support" | null;
      frame_time: string | null;
    };
  }>;
};

const COORDINATE_PRECISION = 6;
const MAXIMUM_REGULAR_SPACING_RATIO = 1.35;
const MAXIMUM_LONGITUDE_SPACING = 90;
const MAXIMUM_MODEL_SUPPORT_RADIUS_NM = 180;
const MODEL_SUPPORT_FOOTPRINT_SEGMENTS = 48;
const EARTH_RADIUS_NM = 3_440.065;

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

function isCompleteRegularGrid(
  points: readonly GlobalWeatherGridPoint[],
): boolean {
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
    return false;
  }
  const combinations = new Set(
    points.map(
      (point) =>
        `${coordinateKey(point.location.latitude)}:${coordinateKey(point.location.longitude)}`,
    ),
  );
  if (combinations.size !== points.length) return false;
  for (const latitude of latitudes) {
    for (const longitude of longitudes) {
      if (
        !combinations.has(
          `${coordinateKey(latitude)}:${coordinateKey(longitude)}`,
        )
      ) {
        return false;
      }
    }
  }
  return true;
}

function greatCircleDistanceNm(
  left: GlobalWeatherGridPoint["location"],
  right: GlobalWeatherGridPoint["location"],
): number {
  const leftLatitude = (left.latitude * Math.PI) / 180;
  const rightLatitude = (right.latitude * Math.PI) / 180;
  const latitudeDelta = rightLatitude - leftLatitude;
  const longitudeDelta = ((right.longitude - left.longitude) * Math.PI) / 180;
  const haversine =
    Math.sin(latitudeDelta / 2) ** 2 +
    Math.cos(leftLatitude) *
      Math.cos(rightLatitude) *
      Math.sin(longitudeDelta / 2) ** 2;
  return (
    2 *
    EARTH_RADIUS_NM *
    Math.asin(Math.sqrt(Math.min(1, Math.max(0, haversine))))
  );
}

function modelSupportRadiusNm(
  point: GlobalWeatherGridPoint,
  points: readonly GlobalWeatherGridPoint[],
): number {
  if (point.provider_extent_radius_nm !== undefined) {
    return point.provider_extent_radius_nm;
  }
  const nearestNeighbourDistance = points.reduce((nearest, candidate) => {
    if (candidate === point) return nearest;
    const distance = greatCircleDistanceNm(point.location, candidate.location);
    return distance > 0 && distance < nearest ? distance : nearest;
  }, Number.POSITIVE_INFINITY);
  // These are indicative, source-local footprints rather than inferred storm
  // boundaries. Half-spacing prevents dense grids from painting beyond their
  // nearest sample, while the fixed cap keeps sparse grids visibly bounded.
  return Math.min(
    MAXIMUM_MODEL_SUPPORT_RADIUS_NM,
    nearestNeighbourDistance / 2,
  );
}

function destinationCoordinate(
  origin: GlobalWeatherGridPoint["location"],
  bearingRadians: number,
  distanceNm: number,
): Coordinate {
  const angularDistance = distanceNm / EARTH_RADIUS_NM;
  const latitude = (origin.latitude * Math.PI) / 180;
  const longitude = (origin.longitude * Math.PI) / 180;
  const destinationLatitude = Math.asin(
    Math.sin(latitude) * Math.cos(angularDistance) +
      Math.cos(latitude) * Math.sin(angularDistance) * Math.cos(bearingRadians),
  );
  const destinationLongitude =
    longitude +
    Math.atan2(
      Math.sin(bearingRadians) * Math.sin(angularDistance) * Math.cos(latitude),
      Math.cos(angularDistance) -
        Math.sin(latitude) * Math.sin(destinationLatitude),
    );
  const longitudeDegrees = (destinationLongitude * 180) / Math.PI;
  return [
    ((longitudeDegrees + 540) % 360) - 180,
    (destinationLatitude * 180) / Math.PI,
  ];
}

function circularSupportFootprint(
  point: GlobalWeatherGridPoint,
  radiusNm: number,
): Coordinate[] | undefined {
  const ring = Array.from(
    { length: MODEL_SUPPORT_FOOTPRINT_SEGMENTS },
    (_, index) =>
      destinationCoordinate(
        point.location,
        (index / MODEL_SUPPORT_FOOTPRINT_SEGMENTS) * Math.PI * 2,
        radiusNm,
      ),
  );
  const closed = [...ring, ring[0]];
  // A single GeoJSON ring spanning the antimeridian would paint across most
  // of the map. Until the renderer can split it safely, leave that extent
  // unknown instead of presenting a false footprint.
  return closed.some(
    (coordinate, index) =>
      index > 0 && Math.abs(coordinate[0] - closed[index - 1][0]) > 180,
  )
    ? undefined
    : closed;
}

function gridCoverageFeatures(
  published: PublishedPluginWeatherLayer,
): WeatherCoverageFeatureCollection["features"] {
  const data = published.layer.data;
  if (data.kind !== "grid") return [];
  const points = displayedGlobalWeatherGridPoints(published.layer);
  const footprintPoints = isCompleteRegularGrid(points)
    ? points
    : points.filter((point) => point.provider_extent_radius_nm !== undefined);
  return footprintPoints.flatMap((point) => {
    const supportRadiusNm = modelSupportRadiusNm(point, points);
    const footprint = circularSupportFootprint(point, supportRadiusNm);
    if (!footprint) return [];
    return [
      {
        type: "Feature" as const,
        geometry: {
          type: "Polygon" as const,
          coordinates: [footprint],
        },
        properties: {
          id: `${published.plugin_id}:${published.layer.id}:${point.id}:support`,
          plugin_id: published.plugin_id,
          layer_title: published.layer.title,
          condition: point.condition,
          coverage_kind: "model_sample_footprint" as const,
          support_radius_nm: supportRadiusNm,
          extent_basis:
            point.provider_extent_radius_nm === undefined
              ? ("sample_support" as const)
              : ("provider_reported" as const),
          frame_time: null,
        },
      },
    ];
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
          support_radius_nm: null,
          extent_basis: null,
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

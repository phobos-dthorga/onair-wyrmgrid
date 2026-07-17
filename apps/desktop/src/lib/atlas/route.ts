import type { Coordinates } from "$lib/atlas/types";
import type { AtlasRouteFeature, AtlasRouteView } from "$lib/dispatch/types";

type RoutePointFeature = {
  type: "Feature";
  geometry: { type: "Point"; coordinates: [number, number] };
  properties: {
    feature_type: "point";
    id: string;
    kind: AtlasRouteFeature["kind"];
    ident: string;
  };
};

type RouteLineFeature = {
  type: "Feature";
  geometry: { type: "MultiLineString"; coordinates: [number, number][][] };
  properties: { feature_type: "route"; id: string };
};

export type AtlasRouteFeatureCollection = {
  type: "FeatureCollection";
  features: Array<RoutePointFeature | RouteLineFeature>;
};

export type AtlasRouteBounds = [
  [west: number, south: number],
  [east: number, north: number],
];

export function orderedRouteFeatures(
  route: AtlasRouteView,
): AtlasRouteFeature[] {
  const byId = new Map(route.features.map((feature) => [feature.id, feature]));
  return route.route_feature_ids.flatMap((id) => {
    const feature = byId.get(id);
    return feature ? [feature] : [];
  });
}

export function findRouteFeature(
  route: AtlasRouteView | undefined,
  featureId: string | null,
): AtlasRouteFeature | null {
  if (!route || !featureId) return null;
  return route.features.find((feature) => feature.id === featureId) ?? null;
}

export function atlasRouteGeoJson(
  route: AtlasRouteView | undefined,
): AtlasRouteFeatureCollection {
  if (!route) return { type: "FeatureCollection", features: [] };

  const routeSegments = resolvedRouteSegments(route);
  const lineFeatures: RouteLineFeature[] = routeSegments.length
    ? [
        {
          type: "Feature",
          geometry: { type: "MultiLineString", coordinates: routeSegments },
          properties: { feature_type: "route", id: route.plan_id },
        },
      ]
    : [];
  const pointFeatures = route.features.flatMap<RoutePointFeature>((feature) =>
    feature.location
      ? [
          {
            type: "Feature",
            geometry: {
              type: "Point",
              coordinates: [
                feature.location.longitude,
                feature.location.latitude,
              ],
            },
            properties: {
              feature_type: "point",
              id: feature.id,
              kind: feature.kind,
              ident: feature.ident,
            },
          },
        ]
      : [],
  );

  return {
    type: "FeatureCollection",
    features: [...lineFeatures, ...pointFeatures],
  };
}

function resolvedRouteSegments(route: AtlasRouteView): [number, number][][] {
  const segments: [number, number][][] = [];
  let current: Coordinates[] = [];
  for (const feature of orderedRouteFeatures(route)) {
    if (feature.location) {
      current.push(feature.location);
      continue;
    }
    if (current.length >= 2) segments.push(unwrapLongitudePath(current));
    current = [];
  }
  if (current.length >= 2) segments.push(unwrapLongitudePath(current));
  return segments;
}

export function atlasRouteBounds(
  route: AtlasRouteView | undefined,
): AtlasRouteBounds | null {
  const locations = route?.features.flatMap((feature) =>
    feature.location ? [feature.location] : [],
  );
  if (!locations?.length) return null;

  const [west, east] = minimalLongitudeRange(
    locations.map((location) => location.longitude),
  );
  const latitudes = locations.map((location) => location.latitude);
  return [
    [west, Math.min(...latitudes)],
    [east, Math.max(...latitudes)],
  ];
}

function unwrapLongitudePath(locations: Coordinates[]): [number, number][] {
  const coordinates: [number, number][] = [];
  for (const location of locations) {
    let longitude = location.longitude;
    const previous = coordinates.at(-1)?.[0];
    if (previous !== undefined) {
      while (longitude - previous > 180) longitude -= 360;
      while (longitude - previous < -180) longitude += 360;
    }
    coordinates.push([longitude, location.latitude]);
  }
  return coordinates;
}

function minimalLongitudeRange(longitudes: number[]): [number, number] {
  if (longitudes.length === 1) return [longitudes[0], longitudes[0]];

  const normalized = longitudes
    .map((longitude) => ((longitude % 360) + 360) % 360)
    .sort((left, right) => left - right);
  let largestGap = -1;
  let gapEndIndex = 0;
  for (let index = 0; index < normalized.length; index += 1) {
    const current = normalized[index];
    const next =
      index === normalized.length - 1
        ? normalized[0] + 360
        : normalized[index + 1];
    const gap = next - current;
    if (gap > largestGap) {
      largestGap = gap;
      gapEndIndex = index;
    }
  }

  const startIndex = (gapEndIndex + 1) % normalized.length;
  let west = normalized[startIndex];
  const span = 360 - largestGap;
  if (west > 180) west -= 360;
  return [west, west + span];
}

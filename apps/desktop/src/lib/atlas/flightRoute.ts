import type { AtlasFlightRoute, AtlasRoutePoint, Coordinates } from "./types";

export type RouteLineFeatureCollection = {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    geometry: { type: "MultiLineString"; coordinates: [number, number][][] };
    properties: { kind: "planned" | "recorded" };
  }>;
};

export type RouteMarkerFeatureCollection = {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    geometry: { type: "Point"; coordinates: [number, number] };
    properties: { label: string; kind: "planned" };
  }>;
};

function coordinate(point: AtlasRoutePoint): [number, number] {
  return [point.location.longitude, point.location.latitude];
}

export function routeSegments(points: AtlasRoutePoint[]): [number, number][][] {
  const segments: [number, number][][] = [];
  let current: [number, number][] = [];
  for (const point of points) {
    if (point.gap_before && current.length > 0) {
      if (current.length > 1) segments.push(current);
      current = [];
    }
    current.push(coordinate(point));
  }
  if (current.length > 1) segments.push(current);
  return segments;
}

export function routeLineFeatures(
  route: AtlasFlightRoute | undefined,
): RouteLineFeatureCollection {
  if (!route) return { type: "FeatureCollection", features: [] };
  const features: RouteLineFeatureCollection["features"] = [];
  const plannedSegments = routeSegments(route.planned?.points ?? []);
  if (plannedSegments.length > 0) {
    features.push({
      type: "Feature",
      geometry: { type: "MultiLineString", coordinates: plannedSegments },
      properties: { kind: "planned" },
    });
  }
  const recordedSegments = routeSegments(route.recorded.points);
  if (recordedSegments.length > 0) {
    features.push({
      type: "Feature",
      geometry: { type: "MultiLineString", coordinates: recordedSegments },
      properties: { kind: "recorded" },
    });
  }
  return { type: "FeatureCollection", features };
}

export function routeMarkerFeatures(
  route: AtlasFlightRoute | undefined,
): RouteMarkerFeatureCollection {
  return {
    type: "FeatureCollection",
    features: (route?.planned?.points ?? []).flatMap((point) =>
      point.label
        ? [
            {
              type: "Feature" as const,
              geometry: {
                type: "Point" as const,
                coordinates: coordinate(point),
              },
              properties: { label: point.label, kind: "planned" as const },
            },
          ]
        : [],
    ),
  };
}

export function routeFitCoordinates(
  route: AtlasFlightRoute | undefined,
): [number, number][] {
  const locations = [
    ...(route?.planned?.points ?? []).map((point) => point.location),
    ...(route?.recorded.points ?? []).map((point) => point.location),
  ];
  if (locations.length < 2)
    return locations.map(({ longitude, latitude }) => [longitude, latitude]);

  const start = longitudeIntervalStart(locations);
  return locations.map(({ longitude, latitude }) => {
    let unwrapped = ((longitude % 360) + 360) % 360;
    if (unwrapped < start) unwrapped += 360;
    return [unwrapped, latitude];
  });
}

function longitudeIntervalStart(locations: Coordinates[]): number {
  const values = locations
    .map(({ longitude }) => ((longitude % 360) + 360) % 360)
    .sort((left, right) => left - right);
  let largestGap = -1;
  let start = values[0] ?? 0;
  for (let index = 0; index < values.length; index += 1) {
    const current = values[index];
    const next =
      index + 1 < values.length ? values[index + 1] : values[0] + 360;
    const gap = next - current;
    if (gap > largestGap) {
      largestGap = gap;
      start = next % 360;
    }
  }
  return start;
}

export function flightRouteSignature(
  route: AtlasFlightRoute | undefined,
): string {
  if (!route) return "";
  return [
    route.session_id,
    ...route.recorded.points.map(
      (point) =>
        `r:${point.source_sequence ?? "?"}:${point.location.longitude}:${point.location.latitude}:${point.gap_before}`,
    ),
    ...(route.planned?.points ?? []).map(
      (point) =>
        `p:${point.label ?? "?"}:${point.location.longitude}:${point.location.latitude}:${point.gap_before}`,
    ),
  ].join("|");
}

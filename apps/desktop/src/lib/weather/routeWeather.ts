import type {
  RouteWeatherAnalysis,
  RouteWeatherLayerAnalysis,
  RouteWeatherSample,
} from "$lib/dispatch/types";
import type { GlobalWeatherCondition } from "$lib/forge/types";

type RouteWeatherLineFeature = {
  type: "Feature";
  geometry: { type: "LineString"; coordinates: [number, number][] };
  properties: {
    id: string;
    layer_id: string;
    provider: string;
    frame_time: string;
    condition: GlobalWeatherCondition;
    support: "supported" | "unavailable";
    temporal_support: "eta_matched" | "current_context" | "unavailable";
    support_distance_nm: number | null;
    estimated_arrival_at: string | null;
    valid_at: string | null;
    time_offset_seconds: number | null;
  };
};

export type RouteWeatherFeatureCollection = {
  type: "FeatureCollection";
  features: RouteWeatherLineFeature[];
};

export function routeWeatherLineFeatures(
  analysis: RouteWeatherAnalysis | undefined,
): RouteWeatherFeatureCollection {
  if (!analysis) return { type: "FeatureCollection", features: [] };
  return {
    type: "FeatureCollection",
    features: analysis.layers.flatMap((layer) => layerLineFeatures(layer)),
  };
}

function layerLineFeatures(
  layer: RouteWeatherLayerAnalysis,
): RouteWeatherLineFeature[] {
  const features: RouteWeatherLineFeature[] = [];
  for (let index = 1; index < layer.samples.length; index += 1) {
    const left = layer.samples[index - 1];
    const right = layer.samples[index];
    if (left.segment_index !== right.segment_index) continue;
    const supported = Boolean(left.source && right.source);
    const source = right.source ?? left.source;
    features.push({
      type: "Feature",
      geometry: {
        type: "LineString",
        coordinates: unwrapPair(left, right),
      },
      properties: {
        id: `${layer.layer_id}:${left.id}:${right.id}`,
        layer_id: layer.layer_id,
        provider: layer.provenance.provider,
        frame_time:
          layer.provenance.generated_at ?? layer.provenance.retrieved_at,
        condition: supported && source ? source.condition : "unknown",
        support: supported ? "supported" : "unavailable",
        temporal_support:
          supported && source ? source.temporal_support : "unavailable",
        support_distance_nm:
          supported && source ? source.support_distance_nm : null,
        estimated_arrival_at: right.estimated_arrival_at ?? null,
        valid_at: supported && source ? (source.valid_at ?? null) : null,
        time_offset_seconds:
          supported && source ? (source.time_offset_seconds ?? null) : null,
      },
    });
  }
  return features;
}

function unwrapPair(
  left: RouteWeatherSample,
  right: RouteWeatherSample,
): [number, number][] {
  let longitude = right.location.longitude;
  while (longitude - left.location.longitude > 180) longitude -= 360;
  while (longitude - left.location.longitude < -180) longitude += 360;
  return [
    [left.location.longitude, left.location.latitude],
    [longitude, right.location.latitude],
  ];
}

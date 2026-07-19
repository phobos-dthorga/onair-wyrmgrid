import { describe, expect, it } from "vitest";
import type { RouteWeatherAnalysis } from "$lib/dispatch/types";
import { routeWeatherLineFeatures } from "./routeWeather";

const analysis: RouteWeatherAnalysis = {
  schema_version: 2,
  plan_id: "plan-1",
  sample_interval_nm: 300,
  maximum_support_distance_nm: 1200,
  maximum_temporal_support_seconds: 10800,
  mapped_route_point_count: 3,
  unresolved_route_point_count: 1,
  timing: {
    availability: "ready",
    departure_basis: "scheduled_off",
    duration_basis: "estimated_enroute",
    departure_at: "2026-07-19T04:00:00Z",
    duration_seconds: 21600,
  },
  availability: "partial",
  layers: [
    {
      layer_id: "global-model",
      title: "Global model",
      availability: "partial",
      provenance: {
        kind: "external_calculation",
        provider: "example.test",
        retrieved_at: "2026-07-19T04:10:00Z",
        generated_at: "2026-07-19T04:00:00Z",
        transformation_version: 1,
        freshness: "current",
      },
      samples: [
        sample("a", 179, 10, 0, "rain", 40),
        sample("b", -179, 11, 0, "cloud", 60),
        { ...sample("c", -170, 12, 1, "snow", 20), source: undefined },
        sample("d", -160, 13, 1, "snow", 20),
      ],
    },
  ],
  radar_contexts: [],
};

describe("route weather presentation", () => {
  it("keeps antimeridian paths short and exposes model support", () => {
    const features = routeWeatherLineFeatures(analysis).features;
    expect(features).toHaveLength(2);
    expect(features[0].geometry.coordinates).toEqual([
      [179, 10],
      [181, 11],
    ]);
    expect(features[0].properties).toMatchObject({
      provider: "example.test",
      frame_time: "2026-07-19T04:00:00Z",
      condition: "cloud",
      support: "supported",
      temporal_support: "eta_matched",
      support_distance_nm: 60,
    });
    expect(features[1].properties.support).toBe("unavailable");
  });

  it("does not invent route weather without an analysis", () => {
    expect(routeWeatherLineFeatures(undefined).features).toEqual([]);
  });
});

function sample(
  id: string,
  longitude: number,
  latitude: number,
  segmentIndex: number,
  condition: "rain" | "cloud" | "snow",
  supportDistance: number,
) {
  return {
    id,
    segment_index: segmentIndex,
    distance_from_origin_nm: 0,
    location: { latitude, longitude },
    estimated_arrival_at: "2026-07-19T05:00:00Z",
    source: {
      point_id: `source-${id}`,
      location: { latitude, longitude },
      support_distance_nm: supportDistance,
      temporal_support: "eta_matched" as const,
      valid_at: "2026-07-19T05:30:00Z",
      time_offset_seconds: 1800,
      condition,
    },
  };
}

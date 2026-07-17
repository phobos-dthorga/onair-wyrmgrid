import { describe, expect, it } from "vitest";
import type { AtlasRouteView } from "$lib/dispatch/types";
import {
  atlasRouteBounds,
  atlasRouteGeoJson,
  findRouteFeature,
  orderedRouteFeatures,
} from "./route";

const route: AtlasRouteView = {
  projection_version: 1,
  plan_id: "plan-1",
  route_feature_ids: ["origin", "fix-0", "fix-1", "destination"],
  mapped_route_feature_count: 3,
  unresolved_route_feature_count: 1,
  provenance: {
    kind: "external_calculation",
    provider: "simbrief",
    retrieved_at: "2026-07-17T00:00:00Z",
    transformation_version: 1,
    freshness: "current",
  },
  features: [
    {
      id: "origin",
      kind: "origin",
      ident: "ORIG",
      availability: "resolved",
      location: { latitude: 10, longitude: 170 },
    },
    {
      id: "fix-0",
      kind: "route_fix",
      ident: "DUP",
      sequence: 0,
      availability: "resolved",
      location: { latitude: 11, longitude: -179 },
    },
    {
      id: "fix-1",
      kind: "route_fix",
      ident: "DUP",
      sequence: 1,
      availability: "location_unavailable",
    },
    {
      id: "destination",
      kind: "destination",
      ident: "DEST",
      availability: "resolved",
      location: { latitude: 12, longitude: -170 },
    },
    {
      id: "alternate",
      kind: "alternate",
      ident: "ALTN",
      availability: "resolved",
      location: { latitude: 8, longitude: 175 },
    },
  ],
};

describe("Atlas route presentation", () => {
  it("preserves authoritative route order and keeps duplicate idents distinct", () => {
    expect(orderedRouteFeatures(route).map((feature) => feature.id)).toEqual([
      "origin",
      "fix-0",
      "fix-1",
      "destination",
    ]);
    expect(findRouteFeature(route, "fix-1")?.availability).toBe(
      "location_unavailable",
    );
  });

  it("plots only resolved facts, breaks at gaps, and retains a short antimeridian path", () => {
    const geoJson = atlasRouteGeoJson(route);
    const line = geoJson.features.find(
      (feature) => feature.geometry.type === "MultiLineString",
    );
    expect(line?.geometry.coordinates).toEqual([
      [
        [170, 10],
        [181, 11],
      ],
    ]);
    expect(
      geoJson.features.filter((feature) => feature.geometry.type === "Point"),
    ).toHaveLength(4);
  });

  it("uses minimal antimeridian-safe bounds including mapped alternates", () => {
    expect(atlasRouteBounds(route)).toEqual([
      [170, 8],
      [190, 12],
    ]);
  });

  it("does not invent a line or bounds when coordinates are unavailable", () => {
    const unavailable = {
      ...route,
      features: route.features.map((feature) => ({
        ...feature,
        availability: "location_unavailable" as const,
        location: undefined,
      })),
    };
    expect(atlasRouteGeoJson(unavailable).features).toEqual([]);
    expect(atlasRouteBounds(unavailable)).toBeNull();
  });
});

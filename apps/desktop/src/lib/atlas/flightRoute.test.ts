import { describe, expect, it } from "vitest";
import type { AtlasFlightRoute, AtlasRoutePoint } from "./types";
import {
  flightRouteSignature,
  routeFitCoordinates,
  routeLineFeatures,
  routeMarkerFeatures,
  routePointCoordinates,
  routeSegments,
} from "./flightRoute";

function point(
  longitude: number,
  latitude = 0,
  gapBefore = false,
): AtlasRoutePoint {
  return {
    location: { longitude, latitude },
    gap_before: gapBefore,
  };
}

function route(recorded: AtlasRoutePoint[]): AtlasFlightRoute {
  return {
    schema_version: 1,
    session_id: "session-1",
    recorded: {
      source_sample_count: recorded.length,
      represented_point_count: recorded.length,
      method: "exact",
      points: recorded,
    },
  };
}

describe("Atlas flight-route projection", () => {
  it("splits line geometry at every explicit evidence gap", () => {
    expect(
      routeSegments([point(1), point(2), point(3, 0, true), point(4)]),
    ).toEqual([
      [
        [1, 0],
        [2, 0],
      ],
      [
        [3, 0],
        [4, 0],
      ],
    ]);
  });

  it("does not emit a line for isolated points around gaps", () => {
    const features = routeLineFeatures(route([point(1), point(2, 0, true)]));
    expect(features.features).toEqual([]);
  });

  it("uses an antimeridian-safe interval for full-route framing", () => {
    const coordinates = routeFitCoordinates(route([point(179), point(-179)]));
    const longitudes = coordinates.map(([longitude]) => longitude);
    expect(Math.max(...longitudes) - Math.min(...longitudes)).toBe(2);
  });

  it("changes its signature when route geometry or gaps change", () => {
    expect(flightRouteSignature(route([point(1), point(2)]))).not.toBe(
      flightRouteSignature(route([point(1), point(2, 0, true)])),
    );
  });

  it("keeps unresolved planned points out of geometry but available to selection", () => {
    const flight = route([]);
    flight.planned = {
      schema_version: 1,
      plan_id: "plan-1",
      origin_icao: "YSSY",
      destination_icao: "NZAA",
      provenance: {
        kind: "external_calculation",
        provider: "simbrief",
        retrieved_at: "2026-07-16T00:00:00Z",
        transformation_version: 1,
        freshness: "current",
      },
      points: [
        {
          id: "route:0000:tesat",
          kind: "route_leg",
          label: "TESAT",
          on_route: true,
          gap_before: false,
        },
      ],
    };

    expect(routeLineFeatures(flight).features).toEqual([]);
    expect(routePointCoordinates(flight, "route:0000:tesat")).toBeUndefined();
    expect(flightRouteSignature(flight)).toContain("route:0000:tesat");
  });

  it("plots alternates without joining them to the filed route", () => {
    const flight = route([]);
    flight.planned = {
      schema_version: 1,
      plan_id: "plan-1",
      origin_icao: "YSSY",
      destination_icao: "NZAA",
      provenance: {
        kind: "external_calculation",
        provider: "simbrief",
        retrieved_at: "2026-07-16T00:00:00Z",
        transformation_version: 1,
        freshness: "current",
      },
      points: [
        {
          id: "origin:yssy",
          kind: "origin",
          label: "YSSY",
          location: { longitude: 151, latitude: -34 },
          on_route: true,
          gap_before: false,
        },
        {
          id: "destination:nzaa",
          kind: "destination",
          label: "NZAA",
          location: { longitude: 175, latitude: -37 },
          on_route: true,
          gap_before: false,
        },
        {
          id: "alternate:0000:nzwn",
          kind: "alternate",
          label: "NZWN",
          location: { longitude: 175, latitude: -41 },
          on_route: false,
          gap_before: false,
        },
      ],
    };

    expect(routeLineFeatures(flight).features[0].geometry.coordinates).toEqual([
      [
        [151, -34],
        [175, -37],
      ],
    ]);
    expect(routeMarkerFeatures(flight).features).toHaveLength(3);
    expect(routePointCoordinates(flight, "alternate:0000:nzwn")).toEqual([
      175, -41,
    ]);
  });
});

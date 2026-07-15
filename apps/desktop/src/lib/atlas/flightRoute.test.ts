import { describe, expect, it } from "vitest";
import type { AtlasFlightRoute, AtlasRoutePoint } from "./types";
import {
  flightRouteSignature,
  routeFitCoordinates,
  routeLineFeatures,
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
});

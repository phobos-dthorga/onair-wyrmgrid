import { describe, expect, it } from "vitest";
import {
  activeAtlasFilterCount,
  atlasSearchItems,
  defaultAtlasFilters,
  filterAtlasItems,
} from "./presentation";
import type { AircraftSummary, FboSummary } from "./types";

const aircraft: AircraftSummary[] = [
  {
    id: "aircraft-1",
    registration: "VH-WYR",
    model: "Turboprop",
    location: { latitude: -37.8, longitude: 144.9 },
    current_airport: {
      id: "ymml",
      icao: "YMML",
      name: "Melbourne",
      location: { latitude: -37.7, longitude: 144.8 },
    },
  },
];
const fbos: FboSummary[] = [
  {
    id: "fbo-1",
    name: "Remote Base",
    airport: {
      id: "yremote",
      icao: "YREM",
      name: "Remote",
      location: null,
    },
  },
];

describe("Atlas exploration", () => {
  it("adapts only received fleet and FBO facts", () => {
    expect(atlasSearchItems(aircraft, fbos)).toMatchObject([
      { id: "aircraft-1", kind: "aircraft", mapped: true },
      { id: "fbo-1", kind: "fbo", mapped: false },
    ]);
  });

  it("searches airport facts and distinguishes unmapped records", () => {
    const items = atlasSearchItems(aircraft, fbos);
    expect(
      filterAtlasItems(items, {
        ...defaultAtlasFilters,
        query: "remote",
        mapping: "unmapped",
      }).map((item) => item.id),
    ).toEqual(["fbo-1"]);
  });

  it("counts non-default Atlas controls", () => {
    expect(
      activeAtlasFilterCount({
        ...defaultAtlasFilters,
        kind: "aircraft",
        sort: "airport",
      }),
    ).toBe(2);
  });
});

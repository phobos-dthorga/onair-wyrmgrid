import { describe, expect, it } from "vitest";
import type { JobSummary } from "$lib/atlas/types";
import {
  activeJobFilterCount,
  defaultJobFilters,
  filterAndSortJobs,
  jobFilterOptions,
  jobPayloadKind,
  jobRouteLabel,
} from "./presentation";

function airport(id: string) {
  return { id, icao: id, name: null, location: null };
}

const cargoJob: JobSummary = {
  id: "cargo",
  mission_type: "Cargo",
  reported_pay: 100,
  expires_at: "2026-07-18T00:00:00Z",
  legs: [
    {
      id: "cargo-leg",
      sequence: 1,
      kind: "cargo",
      departure: airport("YSSY"),
      destination: airport("YMML"),
      current_airport: null,
    },
  ],
};

const passengerJob: JobSummary = {
  id: "passengers",
  mission_type: "Passengers",
  reported_pay: 200,
  legs: [
    {
      id: "passenger-leg",
      sequence: 1,
      kind: "passengers",
      departure: airport("NZAA"),
      destination: airport("NZWN"),
      current_airport: null,
    },
  ],
};

describe("job exploration", () => {
  it("derives filter options only from reported jobs", () => {
    expect(jobFilterOptions([passengerJob, cargoJob])).toEqual({
      missionTypes: ["Cargo", "Passengers"],
      payloadKinds: ["passengers", "cargo"],
      routes: ["NZAA → NZWN", "YSSY → YMML"],
    });
  });

  it("searches route facts and combines explicit filters", () => {
    const filters = {
      ...defaultJobFilters,
      query: "YSSY",
      missionType: "Cargo",
      payload: "cargo" as const,
    };
    expect(filterAndSortJobs([passengerJob, cargoJob], filters)).toEqual([
      cargoJob,
    ]);
    expect(activeJobFilterCount(filters)).toBe(3);
  });

  it("sorts by pay while keeping unavailable facts last", () => {
    const filters = { ...defaultJobFilters, sort: "pay" as const };
    expect(
      filterAndSortJobs([cargoJob, passengerJob], filters).map((job) => job.id),
    ).toEqual(["passengers", "cargo"]);
    expect(activeJobFilterCount(filters)).toBe(1);
  });

  it("restricts plan drilldown to the exact reported route", () => {
    const sameOrigin: JobSummary = {
      ...passengerJob,
      id: "same-origin",
      legs: [
        {
          ...passengerJob.legs[0],
          id: "same-origin-leg",
          departure: airport("YSSY"),
        },
      ],
    };
    const unavailableRoute: JobSummary = {
      ...passengerJob,
      id: "unavailable-route",
      legs: [{ ...passengerJob.legs[0], departure: null, destination: null }],
    };
    const filters = {
      ...defaultJobFilters,
      route: "YSSY → YMML",
    };

    expect(
      filterAndSortJobs([sameOrigin, unavailableRoute, cargoJob], filters),
    ).toEqual([cargoJob]);
    expect(activeJobFilterCount(filters)).toBe(1);
  });

  it("normalizes a plan route before applying the filter", () => {
    expect(jobRouteLabel("  enhv", "efku  ")).toBe("ENHV → EFKU");
    expect(jobRouteLabel("ENHV", undefined)).toBeNull();
  });

  it("does not substitute unrelated jobs when a plan route has no match", () => {
    expect(
      filterAndSortJobs([passengerJob, cargoJob], {
        ...defaultJobFilters,
        route: "ENHV → EFKU",
      }),
    ).toEqual([]);
  });

  it("does not manufacture payload classifications", () => {
    expect(jobPayloadKind({ id: "empty", legs: [] })).toBeUndefined();
  });
});

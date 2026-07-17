import { describe, expect, it } from "vitest";
import type { SimulatorSessionSummary } from "./types";
import {
  activeRecordingFilterCount,
  defaultRecordingFilters,
  filterAndSortRecordings,
  recordingFilterOptions,
} from "./recordingPresentation";

const manual: SimulatorSessionSummary = {
  id: "manual",
  provider_id: "provider.one",
  simulator_family: "MSFS 2024",
  aircraft_title: "Cessna",
  aircraft_registration: "VH-ONE",
  started_at: "2026-07-16T00:00:00Z",
  status: "completed",
  sample_count: 20,
  capture_mode: "manual",
  pinned: false,
  plan_associated: false,
};

const automatic: SimulatorSessionSummary = {
  ...manual,
  id: "automatic",
  aircraft_registration: "VH-TWO",
  started_at: "2026-07-17T00:00:00Z",
  status: "interrupted",
  sample_count: 10,
  capture_mode: "automatic",
  pinned: true,
  plan_associated: true,
};

describe("recording exploration", () => {
  it("derives only observed statuses and capture modes", () => {
    expect(recordingFilterOptions([manual, automatic])).toEqual({
      statuses: ["completed", "interrupted"],
      captureModes: ["manual", "automatic"],
    });
  });

  it("combines search and explicit plan and pin filters", () => {
    const filters = {
      ...defaultRecordingFilters,
      query: "VH-TWO",
      plan: "linked" as const,
      pinned: "pinned" as const,
    };
    expect(filterAndSortRecordings([manual, automatic], filters)).toEqual([
      automatic,
    ]);
    expect(activeRecordingFilterCount(filters)).toBe(3);
  });

  it("sorts newest first by default", () => {
    expect(
      filterAndSortRecordings([manual, automatic], defaultRecordingFilters).map(
        (session) => session.id,
      ),
    ).toEqual(["automatic", "manual"]);
  });

  it("keeps filter clearing available for a sort-only change", () => {
    expect(
      activeRecordingFilterCount({
        ...defaultRecordingFilters,
        sort: "oldest",
      }),
    ).toBe(1);
  });
});

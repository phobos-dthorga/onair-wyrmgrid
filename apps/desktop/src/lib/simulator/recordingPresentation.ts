import {
  compareOptionalDate,
  compareOptionalNumber,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";
import type {
  SimulatorCaptureMode,
  SimulatorRecordingStatus,
  SimulatorSessionSummary,
} from "./types";

export type RecordingPlanFilter = "all" | "linked" | "unlinked";
export type RecordingPinnedFilter = "all" | "pinned" | "unpinned";
export type RecordingSort = "newest" | "oldest" | "samples";

export type RecordingFilters = {
  query: string;
  status: "all" | SimulatorRecordingStatus;
  captureMode: "all" | SimulatorCaptureMode;
  plan: RecordingPlanFilter;
  pinned: RecordingPinnedFilter;
  sort: RecordingSort;
};

export const defaultRecordingFilters: RecordingFilters = {
  query: "",
  status: "all",
  captureMode: "all",
  plan: "all",
  pinned: "all",
  sort: "newest",
};

export function recordingFilterOptions(
  sessions: readonly SimulatorSessionSummary[],
): {
  statuses: SimulatorRecordingStatus[];
  captureModes: SimulatorCaptureMode[];
} {
  return {
    statuses: uniqueReportedValues(sessions.map((session) => session.status)),
    captureModes: uniqueReportedValues(
      sessions.map((session) => session.capture_mode),
    ),
  };
}

export function filterAndSortRecordings(
  sessions: readonly SimulatorSessionSummary[],
  filters: RecordingFilters,
): SimulatorSessionSummary[] {
  return sessions
    .filter((session) => {
      if (
        !matchesQuery(filters.query, [
          session.aircraft_registration,
          session.aircraft_title,
          session.simulator_family,
          session.provider_id,
        ])
      ) {
        return false;
      }
      if (filters.status !== "all" && session.status !== filters.status) {
        return false;
      }
      if (
        filters.captureMode !== "all" &&
        session.capture_mode !== filters.captureMode
      ) {
        return false;
      }
      if (
        filters.plan !== "all" &&
        session.plan_associated !== (filters.plan === "linked")
      ) {
        return false;
      }
      return (
        filters.pinned === "all" ||
        session.pinned === (filters.pinned === "pinned")
      );
    })
    .sort((left, right) => {
      switch (filters.sort) {
        case "oldest":
          return compareOptionalDate(left.started_at, right.started_at);
        case "samples":
          return (
            -compareOptionalNumber(left.sample_count, right.sample_count) ||
            -compareOptionalDate(left.started_at, right.started_at)
          );
        case "newest":
          return -compareOptionalDate(left.started_at, right.started_at);
      }
    });
}

export function activeRecordingFilterCount(filters: RecordingFilters): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.status !== "all",
    filters.captureMode !== "all",
    filters.plan !== "all",
    filters.pinned !== "all",
    filters.sort !== "newest",
  ]);
}

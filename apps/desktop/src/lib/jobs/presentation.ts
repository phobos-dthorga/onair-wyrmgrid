import type { JobSummary } from "$lib/atlas/types";
import {
  compareOptionalDate,
  compareOptionalNumber,
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";

export type JobPayloadFilter = "all" | "cargo" | "passengers" | "mixed";
export type JobExpiryFilter = "all" | "reported" | "unreported";
export type JobSort = "mission" | "route" | "pay" | "expiry" | "legs";

export type JobFilters = {
  query: string;
  missionType: string | null;
  payload: JobPayloadFilter;
  expiry: JobExpiryFilter;
  sort: JobSort;
};

export const defaultJobFilters: JobFilters = {
  query: "",
  missionType: null,
  payload: "all",
  expiry: "all",
  sort: "mission",
};

export function jobRoute(job: JobSummary): string | undefined {
  const first = job.legs[0]?.departure?.icao;
  const last = job.legs.at(-1)?.destination?.icao;
  return first && last ? `${first} → ${last}` : undefined;
}

export function jobPayloadKind(
  job: JobSummary,
): Exclude<JobPayloadFilter, "all"> | undefined {
  const cargo = job.legs.some((leg) => leg.kind === "cargo");
  const passengers = job.legs.some((leg) => leg.kind === "passengers");
  if (cargo && passengers) return "mixed";
  if (cargo) return "cargo";
  if (passengers) return "passengers";
  return undefined;
}

export function jobFilterOptions(jobs: readonly JobSummary[]): {
  missionTypes: string[];
  payloadKinds: Array<Exclude<JobPayloadFilter, "all">>;
} {
  return {
    missionTypes: uniqueReportedValues(
      jobs.map((job) => job.mission_type),
    ).sort(compareOptionalText),
    payloadKinds: uniqueReportedValues(jobs.map(jobPayloadKind)),
  };
}

export function filterAndSortJobs(
  jobs: readonly JobSummary[],
  filters: JobFilters,
): JobSummary[] {
  return jobs
    .filter((job) => {
      if (
        !matchesQuery(filters.query, [
          job.mission_type,
          job.description,
          jobRoute(job),
          ...job.legs.flatMap((leg) => [
            leg.departure?.icao,
            leg.departure?.name,
            leg.destination?.icao,
            leg.destination?.name,
            leg.description,
          ]),
        ])
      ) {
        return false;
      }
      if (
        filters.missionType !== null &&
        job.mission_type !== filters.missionType
      ) {
        return false;
      }
      if (
        filters.payload !== "all" &&
        jobPayloadKind(job) !== filters.payload
      ) {
        return false;
      }
      return (
        filters.expiry === "all" ||
        (filters.expiry === "reported"
          ? job.expires_at !== undefined
          : job.expires_at === undefined)
      );
    })
    .sort((left, right) => {
      switch (filters.sort) {
        case "route":
          return compareOptionalText(jobRoute(left), jobRoute(right));
        case "pay":
          return (
            -compareOptionalNumber(left.reported_pay, right.reported_pay) ||
            compareJobNames(left, right)
          );
        case "expiry":
          return (
            compareOptionalDate(left.expires_at, right.expires_at) ||
            compareJobNames(left, right)
          );
        case "legs":
          return (
            right.legs.length - left.legs.length || compareJobNames(left, right)
          );
        case "mission":
          return compareJobNames(left, right);
      }
    });
}

export function activeJobFilterCount(filters: JobFilters): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.missionType !== null,
    filters.payload !== "all",
    filters.expiry !== "all",
    filters.sort !== "mission",
  ]);
}

function compareJobNames(left: JobSummary, right: JobSummary): number {
  return compareOptionalText(
    left.mission_type ?? left.id,
    right.mission_type ?? right.id,
  );
}

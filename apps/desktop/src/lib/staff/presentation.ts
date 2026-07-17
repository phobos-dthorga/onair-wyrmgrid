import type { StaffMemberSummary } from "$lib/atlas/types";
import {
  compareOptionalNumber,
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";

export type StaffPresenceFilter = "all" | "online" | "offline" | "unreported";
export type StaffBusyFilter = "all" | "reported" | "unreported";
export type StaffSort =
  "name" | "current_airport" | "provider_status" | "qualification_count";

export type StaffFilters = {
  query: string;
  categoryCode: number | null;
  statusCode: number | null;
  presence: StaffPresenceFilter;
  busy: StaffBusyFilter;
  qualificationId: string | null;
  sort: StaffSort;
};

export type StaffFilterOptions = {
  categoryCodes: number[];
  statusCodes: number[];
  qualifications: Array<{ id: string; label: string }>;
};

export const defaultStaffFilters: StaffFilters = {
  query: "",
  categoryCode: null,
  statusCode: null,
  presence: "all",
  busy: "all",
  qualificationId: null,
  sort: "name",
};

const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

export function providerCodeLabel(
  kind: "category" | "status",
  code: number | undefined,
): string {
  return code === undefined ? "Not reported" : `OnAir ${kind} code ${code}`;
}

export function staffSearchText(member: StaffMemberSummary): string {
  return [
    member.display_name,
    member.current_airport?.icao,
    member.current_airport?.name,
    member.home_airport?.icao,
    member.home_airport?.name,
    ...member.class_qualifications.flatMap((qualification) => [
      qualification.short_name,
      qualification.name,
    ]),
  ]
    .filter((value): value is string => Boolean(value))
    .join(" ")
    .toLocaleLowerCase();
}

export function matchesStaffSearch(
  member: StaffMemberSummary,
  query: string,
): boolean {
  return matchesQuery(query, [staffSearchText(member)]);
}

export function staffFilterOptions(
  members: StaffMemberSummary[],
): StaffFilterOptions {
  const categoryCodes = new Set<number>();
  const statusCodes = new Set<number>();
  const qualifications = new Map<string, string>();

  for (const member of members) {
    if (member.category_code !== undefined) {
      categoryCodes.add(member.category_code);
    }
    if (member.status_code !== undefined) statusCodes.add(member.status_code);
    for (const qualification of member.class_qualifications) {
      qualifications.set(
        qualification.aircraft_class_id,
        qualification.short_name ??
          qualification.name ??
          "Unnamed reported class",
      );
    }
  }

  return {
    categoryCodes: uniqueReportedValues(categoryCodes).sort(
      (left, right) => left - right,
    ),
    statusCodes: uniqueReportedValues(statusCodes).sort(
      (left, right) => left - right,
    ),
    qualifications: [...qualifications]
      .map(([id, label]) => ({ id, label }))
      .sort((left, right) => collator.compare(left.label, right.label)),
  };
}

export function filterAndSortStaff(
  members: StaffMemberSummary[],
  filters: StaffFilters,
): StaffMemberSummary[] {
  const filtered = members.filter((member) => {
    if (!matchesStaffSearch(member, filters.query)) return false;
    if (
      filters.categoryCode !== null &&
      member.category_code !== filters.categoryCode
    ) {
      return false;
    }
    if (
      filters.statusCode !== null &&
      member.status_code !== filters.statusCode
    ) {
      return false;
    }
    if (
      filters.presence !== "all" &&
      (filters.presence === "online"
        ? member.is_online !== true
        : filters.presence === "offline"
          ? member.is_online !== false
          : member.is_online !== undefined)
    ) {
      return false;
    }
    if (
      filters.busy !== "all" &&
      (filters.busy === "reported"
        ? member.busy_until === undefined
        : member.busy_until !== undefined)
    ) {
      return false;
    }
    return (
      filters.qualificationId === null ||
      member.class_qualifications.some(
        (qualification) =>
          qualification.aircraft_class_id === filters.qualificationId,
      )
    );
  });

  return [...filtered].sort((left, right) => {
    switch (filters.sort) {
      case "current_airport":
        return compareOptionalText(
          left.current_airport?.icao ?? left.current_airport?.name ?? undefined,
          right.current_airport?.icao ??
            right.current_airport?.name ??
            undefined,
        );
      case "provider_status":
        return compareOptionalNumber(left.status_code, right.status_code);
      case "qualification_count":
        return (
          right.class_qualifications.length -
            left.class_qualifications.length || compareNames(left, right)
        );
      case "name":
        return compareNames(left, right);
    }
  });
}

export function activeStaffFilterCount(filters: StaffFilters): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.categoryCode !== null,
    filters.statusCode !== null,
    filters.presence !== "all",
    filters.busy !== "all",
    filters.qualificationId !== null,
    filters.sort !== "name",
  ]);
}

function compareNames(
  left: StaffMemberSummary,
  right: StaffMemberSummary,
): number {
  return collator.compare(
    left.display_name ?? left.id,
    right.display_name ?? right.id,
  );
}

import {
  compareOptionalDate,
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";
import type { DiagnosticEntry } from "./types";

export type DiagnosticFilters = {
  query: string;
  level: string | null;
  operation: string | null;
  sort: "newest" | "oldest" | "code";
};

export const defaultDiagnosticFilters: DiagnosticFilters = {
  query: "",
  level: null,
  operation: null,
  sort: "newest",
};

export function diagnosticFilterOptions(entries: readonly DiagnosticEntry[]) {
  return {
    levels: uniqueReportedValues(entries.map((entry) => entry.level)).sort(
      compareOptionalText,
    ),
    operations: uniqueReportedValues(
      entries.map((entry) => entry.operation),
    ).sort(compareOptionalText),
  };
}

export function filterDiagnosticEntries(
  entries: readonly DiagnosticEntry[],
  filters: DiagnosticFilters,
): DiagnosticEntry[] {
  const result = entries.filter(
    (entry) =>
      matchesQuery(filters.query, [
        entry.code,
        entry.operation,
        entry.message,
        entry.level,
        entry.plugin_id,
      ]) &&
      (filters.level === null || entry.level === filters.level) &&
      (filters.operation === null || entry.operation === filters.operation),
  );

  return result.sort((left, right) => {
    if (filters.sort === "oldest") {
      return compareOptionalDate(left.occurred_at, right.occurred_at);
    }
    if (filters.sort === "code") {
      return (
        compareOptionalText(left.code, right.code) ||
        -compareOptionalDate(left.occurred_at, right.occurred_at)
      );
    }
    return -compareOptionalDate(left.occurred_at, right.occurred_at);
  });
}

export function activeDiagnosticFilterCount(
  filters: DiagnosticFilters,
): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.level !== null,
    filters.operation !== null,
    filters.sort !== "newest",
  ]);
}

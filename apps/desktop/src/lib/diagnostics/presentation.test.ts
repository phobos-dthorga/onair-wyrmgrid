import { describe, expect, it } from "vitest";
import {
  activeDiagnosticFilterCount,
  defaultDiagnosticFilters,
  diagnosticFilterOptions,
  filterDiagnosticEntries,
} from "./presentation";
import type { DiagnosticEntry } from "./types";

const entries: DiagnosticEntry[] = [
  {
    occurred_at: "2026-07-17T01:00:00Z",
    level: "warning",
    code: "BRIDGE_TIMEOUT",
    operation: "simulator_bridge",
    message: "Provider did not respond.",
  },
  {
    occurred_at: "2026-07-17T02:00:00Z",
    level: "error",
    code: "ONAIR_UNAVAILABLE",
    operation: "onair_sync",
    message: "Provider unavailable.",
  },
];

describe("diagnostic exploration", () => {
  it("derives only levels and operations recorded in the local log", () => {
    expect(diagnosticFilterOptions(entries)).toEqual({
      levels: ["error", "warning"],
      operations: ["onair_sync", "simulator_bridge"],
    });
  });

  it("searches stable diagnostic facts and orders newest first", () => {
    expect(
      filterDiagnosticEntries(entries, {
        ...defaultDiagnosticFilters,
        query: "provider",
      }).map((entry) => entry.code),
    ).toEqual(["ONAIR_UNAVAILABLE", "BRIDGE_TIMEOUT"]);
  });

  it("counts filter state independently of destructive log clearing", () => {
    expect(
      activeDiagnosticFilterCount({
        ...defaultDiagnosticFilters,
        level: "error",
        sort: "code",
      }),
    ).toBe(2);
  });
});

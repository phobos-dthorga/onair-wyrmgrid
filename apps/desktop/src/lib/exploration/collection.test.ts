import { describe, expect, it } from "vitest";
import {
  compareOptionalDate,
  compareOptionalNumber,
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  normalizeQuery,
  selectedOrFirst,
  uniqueReportedValues,
} from "./collection";

describe("collection exploration", () => {
  it("normalizes and matches only supplied facts", () => {
    expect(normalizeQuery("  VH   GFA ")).toBe("vh gfa");
    expect(matchesQuery("gfa", ["VH-GFA", undefined])).toBe(true);
    expect(matchesQuery("invented", ["VH-GFA", undefined])).toBe(false);
  });

  it("derives options only from reported values", () => {
    expect(uniqueReportedValues([3, undefined, 1, 3, null])).toEqual([3, 1]);
  });

  it("counts explicit active-filter flags", () => {
    expect(countActiveFilters([true, false, true])).toBe(2);
  });

  it("reconciles selection against the visible collection", () => {
    const items = [{ id: "a" }, { id: "b" }];
    expect(selectedOrFirst(items, "b", (item) => item.id)).toEqual({ id: "b" });
    expect(selectedOrFirst(items, "missing", (item) => item.id)).toEqual({
      id: "a",
    });
    expect(
      selectedOrFirst<{ id: string }>([], "a", (item) => item.id),
    ).toBeNull();
  });

  it("sorts unavailable values after reported values", () => {
    expect(compareOptionalText(undefined, "A")).toBeGreaterThan(0);
    expect(compareOptionalNumber(undefined, 1)).toBeGreaterThan(0);
    expect(
      compareOptionalDate(undefined, "2026-07-17T00:00:00Z"),
    ).toBeGreaterThan(0);
  });
});

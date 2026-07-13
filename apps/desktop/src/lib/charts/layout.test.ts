import { describe, expect, it } from "vitest";
import {
  categoricalChartHeight,
  readableCategoryLabel,
  usesHorizontalBars,
} from "./layout";

describe("categorical chart layout", () => {
  it("uses horizontal bars when labels or category counts become dense", () => {
    expect(usesHorizontalBars("bar", ["Short", "Another"])).toBe(false);
    expect(usesHorizontalBars("bar", ["Beechcraft King Air 350"])).toBe(true);
    expect(usesHorizontalBars("bar", ["1", "2", "3", "4", "5", "6"])).toBe(
      true,
    );
    expect(usesHorizontalBars("area", Array(12).fill("Observation"))).toBe(
      false,
    );
  });

  it("bounds dense chart height while preserving one row per category", () => {
    expect(categoricalChartHeight(1)).toBe("220px");
    expect(categoricalChartHeight(17)).toBe("376px");
    expect(categoricalChartHeight(100)).toBe("440px");
  });

  it("truncates only unusually long visible labels", () => {
    expect(readableCategoryLabel("Airbus A320-200")).toBe("Airbus A320-200");
    expect(readableCategoryLabel("Beechcraft King Air 350")).toBe(
      "Beechcraft King Air 3…",
    );
  });
});

import type { ChartSpec } from "./types";

const DENSE_CATEGORY_COUNT = 5;
const LONG_CATEGORY_LENGTH = 14;
const MAX_VISIBLE_CATEGORY_LENGTH = 22;
const MIN_CATEGORICAL_CHART_HEIGHT = 220;
const MAX_CATEGORICAL_CHART_HEIGHT = 440;
const HEIGHT_PER_CATEGORY = 18;
const CATEGORICAL_CHART_CHROME = 70;

export function usesHorizontalBars(
  kind: ChartSpec["kind"],
  categories: string[],
): boolean {
  return (
    kind === "bar" &&
    (categories.length > DENSE_CATEGORY_COUNT ||
      categories.some((category) => category.length > LONG_CATEGORY_LENGTH))
  );
}

export function readableCategoryLabel(value: string): string {
  return value.length > MAX_VISIBLE_CATEGORY_LENGTH
    ? `${value.slice(0, MAX_VISIBLE_CATEGORY_LENGTH - 1)}…`
    : value;
}

export function categoricalChartHeight(categoryCount: number): string {
  const contentHeight =
    Math.max(0, categoryCount) * HEIGHT_PER_CATEGORY + CATEGORICAL_CHART_CHROME;
  return `${Math.min(
    MAX_CATEGORICAL_CHART_HEIGHT,
    Math.max(MIN_CATEGORICAL_CHART_HEIGHT, contentHeight),
  )}px`;
}

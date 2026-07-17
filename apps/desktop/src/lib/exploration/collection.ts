export type SearchFact = string | number | boolean | null | undefined;

const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

export function normalizeQuery(value: string): string {
  return value.trim().replaceAll(/\s+/g, " ").toLocaleLowerCase();
}

export function matchesQuery(
  query: string,
  facts: readonly SearchFact[],
): boolean {
  const needle = normalizeQuery(query);
  if (!needle) return true;
  return facts.some((fact) =>
    fact === null || fact === undefined
      ? false
      : String(fact).toLocaleLowerCase().includes(needle),
  );
}

export function uniqueReportedValues<T extends string | number>(
  values: Iterable<T | null | undefined>,
): T[] {
  return [...new Set([...values].filter((value): value is T => value != null))];
}

export function countActiveFilters(flags: readonly boolean[]): number {
  return flags.filter(Boolean).length;
}

export function selectedOrFirst<T>(
  items: readonly T[],
  selectedId: string | null,
  idOf: (item: T) => string,
): T | null {
  return items.find((item) => idOf(item) === selectedId) ?? items[0] ?? null;
}

export function compareOptionalText(
  left: string | null | undefined,
  right: string | null | undefined,
): number {
  if (left == null && right == null) return 0;
  if (left == null) return 1;
  if (right == null) return -1;
  return collator.compare(left, right);
}

export function compareOptionalNumber(
  left: number | null | undefined,
  right: number | null | undefined,
): number {
  if (left == null && right == null) return 0;
  if (left == null) return 1;
  if (right == null) return -1;
  return left - right;
}

export function compareOptionalDate(
  left: string | null | undefined,
  right: string | null | undefined,
): number {
  const leftTime = parseDate(left);
  const rightTime = parseDate(right);
  return compareOptionalNumber(leftTime, rightTime);
}

function parseDate(value: string | null | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? undefined : parsed;
}

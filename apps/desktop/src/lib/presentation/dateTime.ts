export function parseDateTime(value: string | null | undefined): Date | null {
  if (!value) return null;
  const normalized = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}/.test(value)
    ? `${value.replace(" ", "T")}Z`
    : value;
  const parsed = new Date(normalized);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

export function formatLocalDateTime(
  value: string | null | undefined,
  fallback: string,
  options?: Intl.DateTimeFormatOptions,
): string {
  const parsed = parseDateTime(value);
  if (!parsed) return fallback;
  return options
    ? parsed.toLocaleString(undefined, options)
    : parsed.toLocaleString();
}

export const mediumDateShortTime: Intl.DateTimeFormatOptions = {
  dateStyle: "medium",
  timeStyle: "short",
};

export const shortClockTime: Intl.DateTimeFormatOptions = {
  hour: "2-digit",
  minute: "2-digit",
};

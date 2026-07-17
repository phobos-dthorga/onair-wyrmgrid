import { describe, expect, it } from "vitest";
import { formatLocalDateTime, parseDateTime } from "./dateTime";

describe("date and time presentation", () => {
  it("accepts ISO and device-database timestamps", () => {
    expect(parseDateTime("2026-07-17T01:02:03Z")?.toISOString()).toBe(
      "2026-07-17T01:02:03.000Z",
    );
    expect(parseDateTime("2026-07-17 01:02:03")?.toISOString()).toBe(
      "2026-07-17T01:02:03.000Z",
    );
  });

  it("uses the caller's honest unavailable label", () => {
    expect(formatLocalDateTime("not-a-date", "Not reported")).toBe(
      "Not reported",
    );
    expect(formatLocalDateTime(undefined, "Unavailable")).toBe("Unavailable");
  });
});

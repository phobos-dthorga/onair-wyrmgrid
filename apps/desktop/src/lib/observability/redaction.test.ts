import { describe, expect, it } from "vitest";
import type { ErrorEvent } from "@sentry/sveltekit";
import { discardBreadcrumb, sanitizeEvent } from "./redaction";

describe("Sentry event redaction", () => {
  it("removes user content while retaining safe stack coordinates", () => {
    const event = sanitizeEvent({
      message: "api-key=secret-value",
      user: { id: "company-id", username: "pilot" },
      server_name: "developer-machine",
      extra: { database: "C:\\Users\\pilot\\fleet.db" },
      contexts: {
        os: { name: "Windows" },
        device: { name: "Phobos-PC" },
      },
      exception: {
        values: [
          {
            type: "Error",
            value: "company 00000000-0000-0000-0000-000000000000 failed",
            stacktrace: {
              frames: [
                {
                  function: "synchronizeFleet",
                  abs_path: "C:\\Users\\pilot\\src\\secret.ts",
                  context_line: "throw new Error(apiKey)",
                  lineno: 42,
                },
              ],
            },
          },
        ],
      },
    } as unknown as ErrorEvent);

    expect(event.message).toBe(
      "WyrmGrid encountered an unexpected interface failure.",
    );
    expect(event.user).toBeUndefined();
    expect(event.server_name).toBeUndefined();
    expect(event.extra).toBeUndefined();
    expect(event.contexts).toEqual({ os: { name: "Windows" } });
    expect(event.exception?.values?.[0]?.value).not.toContain("00000000");
    expect(event.exception?.values?.[0]?.stacktrace?.frames?.[0]).toMatchObject(
      {
        function: "synchronizeFleet",
        abs_path: "secret.ts",
        lineno: 42,
      },
    );
    expect(
      event.exception?.values?.[0]?.stacktrace?.frames?.[0]?.context_line,
    ).toBeUndefined();
  });

  it("allows only bounded diagnostic codes and application asset URLs", () => {
    const event = sanitizeEvent({
      message: "application.state_unavailable",
      tags: {
        "error.code": "application.state_unavailable",
        "plugin.id": "org.wyrmgrid.provider.open-meteo",
      },
      exception: {
        values: [
          {
            type: "Error",
            stacktrace: {
              frames: [
                {
                  abs_path:
                    "http://tauri.localhost/_app/immutable/main.js?token=secret",
                },
                { abs_path: "https://example.com/company/secret.js" },
              ],
            },
          },
        ],
      },
    } as unknown as ErrorEvent);

    expect(event.message).toBe("application.state_unavailable");
    expect(event.tags).toEqual({
      "error.code": "application.state_unavailable",
    });
    expect(
      event.exception?.values?.[0]?.stacktrace?.frames?.[0]?.abs_path,
    ).toBe("http://tauri.localhost/_app/immutable/main.js");
    expect(
      event.exception?.values?.[0]?.stacktrace?.frames?.[1]?.abs_path,
    ).toBeUndefined();
  });

  it("drops every breadcrumb during the error-only phase", () => {
    expect(
      discardBreadcrumb({ category: "ui.click", message: "Connect" }),
    ).toBeNull();
  });

  it("drops an unbounded or non-machine-owned error code", () => {
    const event = sanitizeEvent({
      message: "secret",
      tags: { "error.code": "plugin secret=value" },
    } as unknown as ErrorEvent);

    expect(event.tags).toEqual({});
    expect(event.message).toBe(
      "WyrmGrid encountered an unexpected interface failure.",
    );
  });
});

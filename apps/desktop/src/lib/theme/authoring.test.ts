import { describe, expect, it } from "vitest";
import { classicTheme } from "./client";
import {
  contrastRatio,
  serialiseThemeDraft,
  themeContrastChecks,
  themeDraftFrom,
  visualDuplicate,
} from "./authoring";

describe("theme authoring preview", () => {
  it("uses the WCAG contrast calculation", () => {
    expect(contrastRatio("#000000", "#FFFFFF")).toBeCloseTo(21, 5);
    expect(contrastRatio("#FFFFFF", "#FFFFFF")).toBeCloseTo(1, 5);
  });

  it("matches the backend thresholds for the built-in classic theme", () => {
    const checks = themeContrastChecks(classicTheme);
    expect(checks.length).toBeGreaterThan(10);
    expect(checks.every((check) => check.passes)).toBe(true);

    const failing = {
      ...classicTheme,
      colors: { ...classicTheme.colors, text: classicTheme.colors.canvas },
    };
    expect(
      themeContrastChecks(failing).find((check) => check.id === "text")?.passes,
    ).toBe(false);
  });

  it("detects a visual duplicate independently of colour casing", () => {
    const draft = themeDraftFrom(classicTheme);
    draft.colors.accent = draft.colors.accent.toLowerCase();
    const available = [
      {
        manifest: classicTheme,
        provenance: { source: "bundled" as const },
      },
    ];

    expect(visualDuplicate(draft, available)?.manifest.id).toBe(
      classicTheme.id,
    );
    draft.colors.accent = "#FFFFFF";
    expect(visualDuplicate(draft, available)).toBeUndefined();

    const unchanged = {
      ...classicTheme,
      colors: { ...classicTheme.colors },
      chart_palette: [...classicTheme.chart_palette],
    };
    expect(visualDuplicate(unchanged, available)?.manifest.id).toBe(
      classicTheme.id,
    );
    unchanged.name = "WyrmGrid Classic Revised";
    expect(visualDuplicate(unchanged, available)).toBeUndefined();
  });

  it("creates an independent custom draft and omits an empty author claim", () => {
    const draft = themeDraftFrom(classicTheme);
    draft.colors.canvas = "#000000";
    draft.chart_palette[0] = "#FFFFFF";

    expect(draft.id).toBe("classic-custom");
    expect(classicTheme.colors.canvas).toBe("#07110F");
    expect(classicTheme.chart_palette[0]).toBe("#73D6AD");
    const exported = JSON.parse(serialiseThemeDraft(draft)) as Record<
      string,
      unknown
    >;
    expect(exported.author).toBeUndefined();
  });
});

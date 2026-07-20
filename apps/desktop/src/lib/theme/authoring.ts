import type { AvailableTheme, ThemeColours, ThemeManifest } from "./types";
import type { TranslationKey } from "$lib/i18n/catalog";

export type ThemeColourRole = {
  key: keyof ThemeColours;
  labelKey: TranslationKey;
};

export type ContrastCheck = {
  id: string;
  labelKey: TranslationKey;
  labelArguments?: Record<string, string | number>;
  ratio: number;
  minimum: number;
  passes: boolean;
};

export const themeColourRoles: readonly ThemeColourRole[] = [
  { key: "canvas", labelKey: "theme-role-canvas" },
  { key: "surface", labelKey: "theme-role-surface" },
  {
    key: "surface_elevated",
    labelKey: "theme-role-surface-elevated",
  },
  {
    key: "surface_soft",
    labelKey: "theme-role-surface-soft",
  },
  { key: "text", labelKey: "theme-role-text" },
  {
    key: "text_muted",
    labelKey: "theme-role-text-muted",
  },
  { key: "line", labelKey: "theme-role-line" },
  { key: "accent", labelKey: "theme-role-accent" },
  {
    key: "highlight",
    labelKey: "theme-role-highlight",
  },
  { key: "danger", labelKey: "theme-role-danger" },
  { key: "success", labelKey: "theme-role-success" },
  {
    key: "map_aircraft",
    labelKey: "theme-role-map-aircraft",
  },
  { key: "map_fbo", labelKey: "theme-role-map-fbo" },
  { key: "map_label", labelKey: "theme-role-map-label" },
  { key: "map_halo", labelKey: "theme-role-map-halo" },
];

const interfaceSurfaceKeys = [
  "canvas",
  "surface",
  "surface_elevated",
  "surface_soft",
] as const satisfies readonly (keyof ThemeColours)[];
const textMinimumContrast = 4.5;
const graphicMinimumContrast = 3;
const dividerMinimumContrast = 1.5;

export function themeDraftFrom(source: ThemeManifest): ThemeManifest {
  const baseId = source.id.replace(/^wyrmgrid-/, "");
  return {
    ...source,
    id: `${baseId}-custom`,
    name: `${source.name} Custom`,
    author: "",
    colors: { ...source.colors },
    chart_palette: [...source.chart_palette],
  };
}

export function contrastRatio(first: string, second: string): number {
  const firstLuminance = relativeLuminance(first);
  const secondLuminance = relativeLuminance(second);
  return (
    (Math.max(firstLuminance, secondLuminance) + 0.05) /
    (Math.min(firstLuminance, secondLuminance) + 0.05)
  );
}

export function themeContrastChecks(theme: ThemeManifest): ContrastCheck[] {
  const checks: ContrastCheck[] = [];
  for (const [key, label] of [
    ["text", "theme-contrast-text"],
    ["text_muted", "theme-contrast-text-muted"],
    ["accent", "theme-contrast-accent"],
    ["highlight", "theme-contrast-highlight"],
    ["danger", "theme-contrast-danger"],
    ["success", "theme-contrast-success"],
  ] as const) {
    checks.push(
      contrastCheck(
        key,
        label,
        Math.min(
          ...interfaceSurfaceKeys.map((surface) =>
            contrastRatio(theme.colors[key], theme.colors[surface]),
          ),
        ),
        textMinimumContrast,
      ),
    );
  }

  checks.push(
    contrastCheck(
      "line",
      "theme-contrast-line",
      contrastRatio(theme.colors.line, theme.colors.surface),
      dividerMinimumContrast,
    ),
    contrastCheck(
      "map-label",
      "theme-contrast-map-label",
      contrastRatio(theme.colors.map_label, theme.colors.map_halo),
      textMinimumContrast,
    ),
  );

  for (const [key, label] of [
    ["map_aircraft", "theme-contrast-map-aircraft"],
    ["map_fbo", "theme-contrast-map-fbo"],
    ["highlight", "theme-contrast-map-highlight"],
  ] as const) {
    checks.push(
      contrastCheck(
        `map-${key}`,
        label,
        contrastRatio(theme.colors[key], theme.colors.map_halo),
        graphicMinimumContrast,
      ),
    );
  }

  theme.chart_palette.forEach((colour, index) => {
    checks.push(
      contrastCheck(
        `chart-${index}`,
        "theme-contrast-chart",
        contrastRatio(colour, theme.colors.surface),
        graphicMinimumContrast,
        { index: index + 1 },
      ),
    );
  });
  return checks;
}

export function visualDuplicate(
  draft: ThemeManifest,
  themes: readonly AvailableTheme[],
): AvailableTheme | undefined {
  return themes.find(
    (theme) =>
      (theme.manifest.id !== draft.id ||
        (theme.manifest.name === draft.name &&
          normalisedAuthor(theme.manifest.author) ===
            normalisedAuthor(draft.author))) &&
      themeColourRoles.every((role) =>
        sameColour(theme.manifest.colors[role.key], draft.colors[role.key]),
      ) &&
      theme.manifest.chart_palette.length === draft.chart_palette.length &&
      theme.manifest.chart_palette.every((colour, index) =>
        sameColour(colour, draft.chart_palette[index]),
      ),
  );
}

export function serialiseThemeDraft(theme: ThemeManifest): string {
  const manifest = {
    ...theme,
    author: theme.author?.trim() || undefined,
  };
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

function contrastCheck(
  id: string,
  labelKey: TranslationKey,
  ratio: number,
  minimum: number,
  labelArguments?: Record<string, string | number>,
): ContrastCheck {
  return {
    id,
    labelKey,
    labelArguments,
    ratio,
    minimum,
    passes: ratio >= minimum,
  };
}

function sameColour(first: string, second: string | undefined): boolean {
  return first.toUpperCase() === second?.toUpperCase();
}

function normalisedAuthor(value: string | undefined): string | undefined {
  return value?.trim() || undefined;
}

function relativeLuminance(colour: string): number {
  if (!/^#[0-9A-Fa-f]{6}$/.test(colour)) return 0;
  const channel = (offset: number): number => {
    const value = Number.parseInt(colour.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045
      ? value / 12.92
      : Math.pow((value + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

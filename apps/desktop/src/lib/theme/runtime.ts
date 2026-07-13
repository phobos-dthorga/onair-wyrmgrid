import { writable } from "svelte/store";
import { classicTheme } from "./client";
import type { ThemeManifest } from "./types";

export const activeTheme = writable<ThemeManifest>(classicTheme);

const colourVariables: Record<keyof ThemeManifest["colors"], string> = {
  canvas: "--color-canvas",
  surface: "--color-surface",
  surface_elevated: "--color-surface-elevated",
  surface_soft: "--color-surface-soft",
  text: "--color-text",
  text_muted: "--color-text-muted",
  line: "--color-line",
  accent: "--color-accent",
  highlight: "--color-highlight",
  danger: "--color-danger",
  success: "--color-success",
  map_aircraft: "--color-map-aircraft",
  map_fbo: "--color-map-fbo",
  map_label: "--color-map-label",
  map_halo: "--color-map-halo",
};

export function applyTheme(theme: ThemeManifest): void {
  activeTheme.set(theme);
  if (typeof document === "undefined") return;

  const root = document.documentElement;
  root.dataset.theme = theme.id;
  for (const [key, variable] of Object.entries(colourVariables)) {
    root.style.setProperty(
      variable,
      theme.colors[key as keyof ThemeManifest["colors"]],
    );
  }

  root.style.setProperty(
    "--color-overlay",
    colourWithAlpha(theme.colors.canvas, 0.82),
  );
  root.style.setProperty(
    "--color-overlay-strong",
    colourWithAlpha(theme.colors.canvas, 0.96),
  );
  root.style.setProperty(
    "--color-surface-translucent",
    colourWithAlpha(theme.colors.surface, 0.94),
  );
  root.style.setProperty(
    "--color-surface-soft-translucent",
    colourWithAlpha(theme.colors.surface_soft, 0.86),
  );
  root.style.setProperty(
    "--color-line-soft",
    colourWithAlpha(theme.colors.line, 0.28),
  );
  root.style.setProperty(
    "--color-line-faint",
    colourWithAlpha(theme.colors.line, 0.18),
  );
  root.style.setProperty(
    "--color-accent-soft",
    colourWithAlpha(theme.colors.accent, 0.12),
  );
  root.style.setProperty(
    "--color-accent-border",
    colourWithAlpha(theme.colors.accent, 0.34),
  );
  root.style.setProperty(
    "--color-accent-glow",
    colourWithAlpha(theme.colors.accent, 0.65),
  );
  root.style.setProperty(
    "--color-highlight-soft",
    colourWithAlpha(theme.colors.highlight, 0.12),
  );
  root.style.setProperty(
    "--color-highlight-border",
    colourWithAlpha(theme.colors.highlight, 0.32),
  );
  root.style.setProperty(
    "--color-danger-soft",
    colourWithAlpha(theme.colors.danger, 0.1),
  );
  root.style.setProperty(
    "--color-danger-border",
    colourWithAlpha(theme.colors.danger, 0.34),
  );
}

export function colourWithAlpha(hex: string, alpha: number): string {
  const red = Number.parseInt(hex.slice(1, 3), 16);
  const green = Number.parseInt(hex.slice(3, 5), 16);
  const blue = Number.parseInt(hex.slice(5, 7), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

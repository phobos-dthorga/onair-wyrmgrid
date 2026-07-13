import { colourWithAlpha } from "$lib/theme/runtime";
import type { ThemeManifest } from "$lib/theme/types";

export function chartPresentation(theme: ThemeManifest) {
  return {
    colours: theme.chart_palette,
    text: theme.colors.text,
    muted: theme.colors.text_muted,
    line: colourWithAlpha(theme.colors.line, 0.18),
    tooltipBackground: colourWithAlpha(theme.colors.canvas, 0.96),
    tooltipBorder: colourWithAlpha(theme.colors.line, 0.28),
  };
}

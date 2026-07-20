import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { save } from "@tauri-apps/plugin-dialog";
import type { ThemeExport, ThemeManifest, ThemeStatus } from "./types";

const themeFileFilter = {
  name: "WyrmGrid theme",
  extensions: ["json"],
};

export const classicTheme: ThemeManifest = {
  schema_version: 1,
  id: "wyrmgrid-classic",
  name: "WyrmGrid Classic",
  author: "WyrmGrid",
  colors: {
    canvas: "#07110F",
    surface: "#0A1916",
    surface_elevated: "#102520",
    surface_soft: "#172F29",
    text: "#E9F1EF",
    text_muted: "#A7B8B2",
    line: "#526A62",
    accent: "#73D6AD",
    highlight: "#D5AE5F",
    danger: "#ED8074",
    success: "#73D6AD",
    map_aircraft: "#D5AE5F",
    map_fbo: "#73D6AD",
    map_label: "#E9F1EF",
    map_halo: "#07110F",
  },
  chart_palette: ["#73D6AD", "#D5AE5F", "#72A7CF", "#CF7B73", "#A88BD4"],
};

export const browserThemeStatus: ThemeStatus = {
  selected_theme_id: classicTheme.id,
  active_theme: classicTheme,
  themes: [
    {
      manifest: classicTheme,
      provenance: { source: "bundled" },
    },
  ],
};

export async function loadThemeStatus(): Promise<ThemeStatus> {
  return isDesktopRuntime()
    ? invokeDesktop<ThemeStatus>("theme_status")
    : browserThemeStatus;
}

export async function selectTheme(themeId: string): Promise<ThemeStatus> {
  return invokeDesktop<ThemeStatus>("select_theme", { themeId });
}

export async function importTheme(manifestJson: string): Promise<ThemeStatus> {
  return invokeDesktop<ThemeStatus>("import_theme", { manifestJson });
}

export async function exportTheme(themeId: string): Promise<ThemeExport> {
  return invokeDesktop<ThemeExport>("export_theme", { themeId });
}

export function chooseThemeFileDestination(
  suggestedFilename: string,
): Promise<string | null> {
  return save({
    defaultPath: suggestedFilename,
    filters: [themeFileFilter],
  });
}

export function saveThemeExport(
  themeId: string,
  destination: string,
): Promise<void> {
  return invokeDesktop<void>("save_theme_export", { themeId, destination });
}

export function saveThemeDraft(
  manifestJson: string,
  destination: string,
): Promise<void> {
  return invokeDesktop<void>("save_theme_draft", {
    manifestJson,
    destination,
  });
}

export async function deleteTheme(themeId: string): Promise<ThemeStatus> {
  return invokeDesktop<ThemeStatus>("delete_theme", { themeId });
}

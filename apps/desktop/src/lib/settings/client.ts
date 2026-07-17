import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { aviationDisplayPreferences, type DisplayPreferences } from "./types";

const PREVIEW_STORAGE_KEY = "wyrmgrid.preview.display-preferences";

export async function loadDisplayPreferences(): Promise<DisplayPreferences> {
  if (isDesktopRuntime()) {
    return invokeDesktop<DisplayPreferences>("display_preferences");
  }
  try {
    return validatePreferences(
      JSON.parse(localStorage.getItem(PREVIEW_STORAGE_KEY) ?? "null"),
    );
  } catch {
    return aviationDisplayPreferences;
  }
}

export async function saveDisplayPreferences(
  preferences: DisplayPreferences,
): Promise<DisplayPreferences> {
  if (isDesktopRuntime()) {
    return invokeDesktop<DisplayPreferences>("update_display_preferences", {
      preferences,
    });
  }
  const validated = validatePreferences(preferences);
  localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(validated));
  return validated;
}

function validatePreferences(value: unknown): DisplayPreferences {
  if (!value || typeof value !== "object") return aviationDisplayPreferences;
  const candidate = value as Partial<DisplayPreferences>;
  const responsiveSurfaces = candidate.responsive_surfaces ?? true;
  if (
    !["feet", "metres"].includes(candidate.altitude_unit ?? "") ||
    ![
      "knots",
      "miles_per_hour",
      "kilometres_per_hour",
      "metres_per_second",
    ].includes(candidate.speed_unit ?? "") ||
    !["pounds", "kilograms"].includes(candidate.weight_unit ?? "") ||
    ![
      "pounds",
      "kilograms",
      "us_gallons",
      "imperial_gallons",
      "litres",
    ].includes(candidate.fuel_unit ?? "") ||
    typeof responsiveSurfaces !== "boolean"
  ) {
    return aviationDisplayPreferences;
  }
  return {
    ...(candidate as Omit<DisplayPreferences, "responsive_surfaces">),
    responsive_surfaces: responsiveSurfaces,
  };
}

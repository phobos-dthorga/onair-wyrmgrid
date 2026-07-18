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

export function validatePreferences(value: unknown): DisplayPreferences {
  if (!value || typeof value !== "object") return aviationDisplayPreferences;
  const candidate = value as Partial<DisplayPreferences>;
  const altitudeUnit = candidate.altitude_unit;
  const speedUnit = candidate.speed_unit;
  const weightUnit = candidate.weight_unit;
  const fuelUnit = candidate.fuel_unit;
  const responsiveSurfaces = candidate.responsive_surfaces ?? true;
  const weatherRenderingProfile =
    candidate.weather_rendering_profile ?? "enhanced";
  if (
    !altitudeUnit ||
    !["feet", "metres"].includes(altitudeUnit) ||
    !speedUnit ||
    ![
      "knots",
      "miles_per_hour",
      "kilometres_per_hour",
      "metres_per_second",
    ].includes(speedUnit) ||
    !weightUnit ||
    !["pounds", "kilograms"].includes(weightUnit) ||
    !fuelUnit ||
    ![
      "pounds",
      "kilograms",
      "us_gallons",
      "imperial_gallons",
      "litres",
    ].includes(fuelUnit) ||
    typeof responsiveSurfaces !== "boolean" ||
    !["compatibility", "enhanced"].includes(weatherRenderingProfile)
  ) {
    return aviationDisplayPreferences;
  }
  return {
    altitude_unit: altitudeUnit,
    speed_unit: speedUnit,
    weight_unit: weightUnit,
    fuel_unit: fuelUnit,
    responsive_surfaces: responsiveSurfaces,
    weather_rendering_profile: weatherRenderingProfile,
  };
}

import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";

export const automaticSyncOptions = [0, 15, 30, 60, 120] as const;
const LEGACY_AUTOMATIC_SYNC_STORAGE_KEY =
  "wyrmgrid.atlas.automatic-sync-minutes";

export type AtlasLayerVisibility = {
  daylight: boolean;
  regions: boolean;
  route: boolean;
  fleet: boolean;
  fbos: boolean;
  airport_weather: boolean;
  global_weather: boolean;
  weather_coverage: boolean;
  plugin_layers: boolean;
};

export type AtlasView = {
  longitude: number;
  latitude: number;
  zoom: number;
  bearing: number;
  pitch: number;
};

export type AtlasPreferences = {
  automatic_sync_minutes: number;
  layers: AtlasLayerVisibility;
  restore_last_view: boolean;
  last_view?: AtlasView;
};

export const defaultAtlasPreferences: AtlasPreferences = {
  automatic_sync_minutes: 30,
  layers: {
    daylight: true,
    regions: true,
    route: true,
    fleet: true,
    fbos: true,
    airport_weather: true,
    global_weather: true,
    weather_coverage: true,
    plugin_layers: true,
  },
  restore_last_view: false,
};

export function automaticSyncDelayMs(minutes: number): number | undefined {
  if (
    !automaticSyncOptions.includes(
      minutes as (typeof automaticSyncOptions)[number],
    ) ||
    minutes === 0
  ) {
    return undefined;
  }
  return minutes * 60 * 1000;
}

let previewPreferences = clonePreferences(defaultAtlasPreferences);

export async function loadAtlasPreferences(): Promise<AtlasPreferences> {
  if (isDesktopRuntime()) {
    let preferences = validateAtlasPreferences(
      await invokeDesktop<AtlasPreferences>("atlas_preferences"),
    );
    const legacyValue = Number.parseInt(
      localStorage.getItem(LEGACY_AUTOMATIC_SYNC_STORAGE_KEY) ?? "",
      10,
    );
    if (
      automaticSyncOptions.includes(
        legacyValue as (typeof automaticSyncOptions)[number],
      )
    ) {
      preferences = validateAtlasPreferences(
        await invokeDesktop<AtlasPreferences>("update_atlas_preferences", {
          preferences: {
            ...preferences,
            automatic_sync_minutes: legacyValue,
          },
        }),
      );
      localStorage.removeItem(LEGACY_AUTOMATIC_SYNC_STORAGE_KEY);
    }
    return preferences;
  }
  return clonePreferences(previewPreferences);
}

export async function saveAtlasPreferences(
  preferences: AtlasPreferences,
): Promise<AtlasPreferences> {
  const validated = validateAtlasPreferences(preferences);
  if (isDesktopRuntime()) {
    return validateAtlasPreferences(
      await invokeDesktop<AtlasPreferences>("update_atlas_preferences", {
        preferences: validated,
      }),
    );
  }
  previewPreferences = {
    ...clonePreferences(validated),
    last_view: validated.restore_last_view
      ? previewPreferences.last_view
      : undefined,
  };
  return clonePreferences(previewPreferences);
}

export async function saveAtlasView(
  view: AtlasView,
): Promise<AtlasPreferences> {
  const validated = validateAtlasView(view);
  if (isDesktopRuntime()) {
    return validateAtlasPreferences(
      await invokeDesktop<AtlasPreferences>("update_atlas_view", {
        view: validated,
      }),
    );
  }
  if (previewPreferences.restore_last_view) {
    previewPreferences = { ...previewPreferences, last_view: validated };
  }
  return clonePreferences(previewPreferences);
}

export function validateAtlasPreferences(value: unknown): AtlasPreferences {
  if (!value || typeof value !== "object") {
    return clonePreferences(defaultAtlasPreferences);
  }
  const candidate = value as Partial<AtlasPreferences>;
  const layers = candidate.layers as Partial<AtlasLayerVisibility> | undefined;
  if (
    !automaticSyncOptions.includes(
      candidate.automatic_sync_minutes as (typeof automaticSyncOptions)[number],
    ) ||
    !layers ||
    !Object.values(layers).every((visible) => typeof visible === "boolean") ||
    Object.keys(defaultAtlasPreferences.layers).some(
      (key) => typeof layers[key as keyof AtlasLayerVisibility] !== "boolean",
    ) ||
    typeof candidate.restore_last_view !== "boolean"
  ) {
    return clonePreferences(defaultAtlasPreferences);
  }
  let lastView: AtlasView | undefined;
  if (candidate.restore_last_view && candidate.last_view) {
    try {
      lastView = validateAtlasView(candidate.last_view);
    } catch {
      return clonePreferences(defaultAtlasPreferences);
    }
  }
  return {
    automatic_sync_minutes: candidate.automatic_sync_minutes as number,
    layers: { ...(layers as AtlasLayerVisibility) },
    restore_last_view: candidate.restore_last_view,
    last_view: lastView,
  };
}

export function validateAtlasView(value: AtlasView): AtlasView {
  if (
    !Number.isFinite(value.longitude) ||
    !Number.isFinite(value.latitude) ||
    !Number.isFinite(value.zoom) ||
    !Number.isFinite(value.bearing) ||
    !Number.isFinite(value.pitch) ||
    value.latitude < -90 ||
    value.latitude > 90 ||
    value.zoom < 0 ||
    value.zoom > 24 ||
    value.pitch < 0 ||
    value.pitch > 85
  ) {
    throw new Error("That Atlas view is outside the supported range.");
  }
  return {
    ...value,
    longitude: normalizeAngle(value.longitude),
    bearing: normalizeAngle(value.bearing),
  };
}

function normalizeAngle(value: number): number {
  return value >= -180 && value <= 180
    ? value
    : ((((value + 180) % 360) + 360) % 360) - 180;
}

function clonePreferences(preferences: AtlasPreferences): AtlasPreferences {
  return {
    ...preferences,
    layers: { ...preferences.layers },
    last_view: preferences.last_view ? { ...preferences.last_view } : undefined,
  };
}

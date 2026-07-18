import type { TranslationKey } from "$lib/i18n/catalog";
import { displayPresets, type WeatherRenderingProfile } from "./types";

export type DisplayPreset = keyof typeof displayPresets;

export const displayPresetMessageKeys = {
  aviation: "settings-preset-aviation",
  imperial: "settings-preset-imperial",
  metric: "settings-preset-metric",
  si: "settings-preset-si",
} as const satisfies Record<DisplayPreset, TranslationKey>;

export const weatherProfileDetailMessageKeys = {
  compatibility: "settings-weather-profile-compatibility-detail",
  enhanced: "settings-weather-profile-enhanced-detail",
  cinematic: "settings-weather-profile-cinematic-detail",
} as const satisfies Record<WeatherRenderingProfile, TranslationKey>;

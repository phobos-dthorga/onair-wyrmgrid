import type { TranslationKey } from "$lib/i18n/catalog";

export type PluginSettingPresentation = {
  label: TranslationKey;
  detail: TranslationKey;
};

const settingPresentation: Readonly<Record<string, PluginSettingPresentation>> =
  {
    forecast_refresh_minutes: {
      label: "forge-setting-forecast-refresh",
      detail: "forge-setting-forecast-refresh-detail",
    },
    radar_refresh_minutes: {
      label: "forge-setting-radar-refresh",
      detail: "forge-setting-radar-refresh-detail",
    },
  };

const refreshChoiceKeys: Readonly<Record<string, TranslationKey>> = {
  "5": "forge-setting-refresh-5",
  "10": "forge-setting-refresh-10",
  "15": "forge-setting-refresh-15",
  "30": "forge-setting-refresh-30",
  "60": "forge-setting-refresh-60",
  "120": "forge-setting-refresh-120",
};

export function pluginSettingPresentation(
  settingKey: string,
): PluginSettingPresentation {
  return (
    settingPresentation[settingKey] ?? {
      label: "forge-setting-unknown",
      detail: "forge-setting-unknown-detail",
    }
  );
}

export function pluginSettingChoiceKey(value: string): TranslationKey {
  return refreshChoiceKeys[value] ?? "forge-setting-refresh-unknown";
}

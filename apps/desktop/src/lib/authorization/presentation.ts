import type { TranslationKey } from "$lib/i18n/catalog";

export const capabilityTranslationKeys: Readonly<
  Record<string, TranslationKey>
> = {
  on_air_company_read: "security-capability-company-read",
  on_air_fleet_read: "security-capability-fleet-read",
  on_air_jobs_read: "security-capability-jobs-read",
  on_air_finance_read: "security-capability-finance-read",
  map_layers_publish: "security-capability-map-publish",
  charts_publish: "security-capability-charts-publish",
  notifications_create: "security-capability-notifications-create",
  plugin_storage: "security-capability-plugin-storage",
  simulator_telemetry_read: "security-capability-simulator-read",
  external_network: "security-capability-external-network",
  weather_data_publish: "security-capability-weather-publish",
};

export function capabilityTranslationKey(
  capability: string,
): TranslationKey | null {
  return capabilityTranslationKeys[capability] ?? null;
}

export function lifetimeTranslationKey(
  lifetime: "once" | "session" | "standing",
): TranslationKey {
  return lifetimeTranslationKeys[lifetime];
}

const lifetimeTranslationKeys = {
  once: "security-lifetime-once",
  session: "security-lifetime-session",
  standing: "security-lifetime-standing",
} as const satisfies Record<"once" | "session" | "standing", TranslationKey>;

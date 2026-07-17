export const capabilityTranslationKeys: Readonly<Record<string, string>> = {
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
};

export function capabilityTranslationKey(capability: string): string | null {
  return capabilityTranslationKeys[capability] ?? null;
}

export function lifetimeTranslationKey(
  lifetime: "once" | "session" | "standing",
): string {
  return `security-lifetime-${lifetime}`;
}

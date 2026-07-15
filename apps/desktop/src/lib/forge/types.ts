import type { Coordinates, FleetSnapshot } from "$lib/atlas/types";

export type PluginPermission =
  | "on_air_company_read"
  | "on_air_fleet_read"
  | "on_air_jobs_read"
  | "on_air_finance_read"
  | "map_layers_publish"
  | "charts_publish"
  | "notifications_create"
  | "plugin_storage"
  | "simulator_telemetry_read"
  | "external_network";

export type PluginProcessState =
  "stopped" | "starting" | "running" | "stopping" | "failed";

export type AuthorizationGrantLifetime = "once" | "session" | "standing";

export type PluginView = {
  id: string;
  name: string;
  version: string;
  author: string;
  runtime: "python" | null;
  requested_permissions: PluginPermission[];
  granted_permissions: PluginPermission[];
  grant_lifetime?: AuthorizationGrantLifetime;
  state: PluginProcessState;
  published_layer_count: number;
  last_error?: string;
};

export type PluginMapPoint = {
  id: string;
  label: string;
  location: Coordinates;
};

export type PluginMapLayer = {
  id: string;
  title: string;
  points: PluginMapPoint[];
  provenance: FleetSnapshot["provenance"];
};

export type PublishedPluginLayer = {
  plugin_id: string;
  plugin_name: string;
  layer: PluginMapLayer;
};

export type PluginHostView = {
  available: boolean;
  notice?: string;
  plugins: PluginView[];
  layers: PublishedPluginLayer[];
};

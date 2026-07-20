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
  | "external_network"
  | "weather_data_publish";

export type WeatherCapability =
  "airport_reports" | "forecast_grid" | "radar_tiles";

export type PluginProcessState =
  "stopped" | "starting" | "running" | "stopping" | "failed";

export type AuthorizationGrantLifetime = "once" | "session" | "standing";

export type PluginSettingChoice = {
  value: string;
};

export type PluginSettingView = {
  key: string;
  value: string;
  choices: PluginSettingChoice[];
};

export type PluginView = {
  id: string;
  name: string;
  version: string;
  author: string;
  runtime: "python" | null;
  weather_capabilities: WeatherCapability[];
  network_origins: string[];
  requested_permissions: PluginPermission[];
  granted_permissions: PluginPermission[];
  grant_lifetime?: AuthorizationGrantLifetime;
  start_with_wyrmgrid: boolean;
  configuration: PluginSettingView[];
  state: PluginProcessState;
  published_layer_count: number;
  published_weather_layer_count: number;
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

export type GlobalWeatherCondition =
  | "clear"
  | "cloud"
  | "rain"
  | "snow"
  | "convective"
  | "obscuration"
  | "unknown";

export type GlobalWeatherGridPoint = {
  id: string;
  location: Coordinates;
  valid_at?: string;
  condition: GlobalWeatherCondition;
  temperature_c?: number;
  precipitation_mm?: number;
  cloud_cover_percent?: number;
  wind_direction_degrees?: number;
  wind_speed_kt?: number;
  provider_weather_code?: number;
  provider_extent_radius_nm?: number;
};

export type GlobalWeatherTimeScope = {
  kind: "current_forecast" | "historical_model" | "archived_forecast";
  target_at: string;
  starts_at: string;
  ends_at: string;
};

export type GlobalWeatherRasterTile = {
  zoom: number;
  x: number;
  y: number;
  png_base64: string;
  coverage_png_base64?: string;
};

export type GlobalWeatherLayer = {
  schema_version: number;
  id: string;
  title: string;
  time_scope?: GlobalWeatherTimeScope;
  data:
    | { kind: "grid"; points: GlobalWeatherGridPoint[] }
    | {
        kind: "raster_tiles";
        frame_time: string;
        tiles: GlobalWeatherRasterTile[];
      };
  provenance: import("$lib/operational/types").OperationalProvenance;
};

export type PublishedPluginWeatherLayer = {
  plugin_id: string;
  plugin_name: string;
  layer: GlobalWeatherLayer;
};

export type PluginHostView = {
  available: boolean;
  notice?: string;
  plugins: PluginView[];
  layers: PublishedPluginLayer[];
  weather_layers: PublishedPluginWeatherLayer[];
};

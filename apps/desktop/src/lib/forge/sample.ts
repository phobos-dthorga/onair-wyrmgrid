import type { PluginHostView, PluginView } from "./types";

const requestedPermissions = [
  "on_air_fleet_read",
  "map_layers_publish",
  "weather_data_publish",
] as const;

const previewPlugin: PluginView = {
  id: "org.wyrmgrid.example.fleet-locations",
  name: "Fleet Locations",
  version: "0.1.0",
  author: "WyrmGrid contributors",
  runtime: "python",
  weather_capabilities: ["forecast_grid"],
  network_origins: [],
  requested_permissions: [...requestedPermissions],
  granted_permissions: [],
  start_with_wyrmgrid: false,
  state: "stopped",
  published_layer_count: 0,
  published_weather_layer_count: 0,
};

export const forgePreviewStopped: PluginHostView = {
  available: true,
  plugins: [previewPlugin],
  layers: [],
  weather_layers: [],
};

export const forgePreviewApproved: PluginHostView = {
  available: true,
  plugins: [
    {
      ...previewPlugin,
      granted_permissions: [...requestedPermissions],
    },
  ],
  layers: [],
  weather_layers: [],
};

export const forgePreviewRunning: PluginHostView = {
  available: true,
  plugins: [
    {
      ...previewPlugin,
      granted_permissions: [...requestedPermissions],
      state: "running",
      published_layer_count: 1,
      published_weather_layer_count: 1,
    },
  ],
  layers: [
    {
      plugin_id: previewPlugin.id,
      plugin_name: previewPlugin.name,
      layer: {
        id: "fleet-locations",
        title: "Fleet locations",
        points: [
          {
            id: "11111111-1111-4111-8111-111111111111",
            label: "WYR-101 · Example Turboprop",
            location: { latitude: -37.8136, longitude: 144.9631 },
          },
          {
            id: "22222222-2222-4222-8222-222222222222",
            label: "WYR-202 · Example Jet",
            location: { latitude: 51.5072, longitude: -0.1276 },
          },
          {
            id: "33333333-3333-4333-8333-333333333333",
            label: "WYR-303 · Example Bush Aircraft",
            location: { latitude: 61.2181, longitude: -149.9003 },
          },
        ],
        provenance: {
          kind: "calculated",
          source: "Fleet Locations example plugin",
          observed_at: "2026-07-14T00:00:00Z",
        },
      },
    },
  ],
  weather_layers: [
    {
      plugin_id: previewPlugin.id,
      plugin_name: previewPlugin.name,
      layer: {
        schema_version: 1,
        id: "weather-pattern-reference",
        title: "Weather pattern reference",
        data: {
          kind: "grid",
          points: [
            {
              id: "cloud",
              location: { latitude: 20, longitude: -30 },
              condition: "cloud",
            },
            {
              id: "rain",
              location: { latitude: 20, longitude: 15 },
              condition: "rain",
            },
            {
              id: "snow",
              location: { latitude: 20, longitude: 60 },
              condition: "snow",
            },
            {
              id: "convective",
              location: { latitude: -20, longitude: -30 },
              condition: "convective",
            },
            {
              id: "obscuration",
              location: { latitude: -20, longitude: 15 },
              condition: "obscuration",
            },
            {
              id: "clear",
              location: { latitude: -20, longitude: 60 },
              condition: "clear",
            },
          ],
        },
        provenance: {
          kind: "external_calculation",
          provider: "WyrmGrid illustrative preview",
          generated_at: "2026-07-19T00:00:00Z",
          retrieved_at: "2026-07-19T00:00:00Z",
          transformation_version: 1,
          freshness: "current",
        },
      },
    },
  ],
};

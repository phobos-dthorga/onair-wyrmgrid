import { describe, expect, it } from "vitest";
import {
  activeForgeFilterCount,
  defaultForgeFilters,
  filterForgePlugins,
  forgeFilterOptions,
} from "./presentation";
import type { PluginView } from "./types";

const plugins: PluginView[] = [
  {
    id: "org.example.ready",
    name: "Ready Atlas",
    version: "1.0.0",
    author: "Example",
    runtime: "python",
    weather_capabilities: [],
    network_origins: [],
    requested_permissions: ["map_layers_publish"],
    granted_permissions: ["map_layers_publish"],
    start_with_wyrmgrid: false,
    configuration: [],
    state: "running",
    published_layer_count: 1,
    published_weather_layer_count: 0,
  },
  {
    id: "org.example.review",
    name: "Fleet Review",
    version: "1.0.0",
    author: "Example",
    runtime: null,
    weather_capabilities: [],
    network_origins: [],
    requested_permissions: ["on_air_fleet_read"],
    granted_permissions: [],
    start_with_wyrmgrid: false,
    configuration: [],
    state: "stopped",
    published_layer_count: 0,
    published_weather_layer_count: 0,
  },
];

describe("Forge exploration", () => {
  it("derives filters only from installed plugin facts", () => {
    expect(forgeFilterOptions(plugins)).toEqual({
      states: ["running", "stopped"],
      capabilities: ["map_layers_publish", "on_air_fleet_read"],
    });
  });

  it("searches capabilities and filters permission review", () => {
    expect(
      filterForgePlugins(plugins, {
        ...defaultForgeFilters,
        query: "fleet",
        access: "review",
      }).map((plugin) => plugin.id),
    ).toEqual(["org.example.review"]);
  });

  it("counts changed presentation controls", () => {
    expect(
      activeForgeFilterCount({
        ...defaultForgeFilters,
        state: "running",
        capability: "map_layers_publish",
      }),
    ).toBe(2);
  });
});

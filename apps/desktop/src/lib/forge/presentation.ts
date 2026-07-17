import {
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
  uniqueReportedValues,
} from "$lib/exploration/collection";
import type { PluginPermission, PluginProcessState, PluginView } from "./types";

export type ForgeFilters = {
  query: string;
  state: PluginProcessState | "all";
  access: "all" | "approved" | "review";
  capability: PluginPermission | null;
  sort: "name" | "state";
};

export const defaultForgeFilters: ForgeFilters = {
  query: "",
  state: "all",
  access: "all",
  capability: null,
  sort: "name",
};

export function allRequestedCapabilitiesGranted(plugin: PluginView): boolean {
  return plugin.requested_permissions.every((permission) =>
    plugin.granted_permissions.includes(permission),
  );
}

export function forgeFilterOptions(plugins: readonly PluginView[]) {
  return {
    states: uniqueReportedValues(plugins.map((plugin) => plugin.state)).sort(
      compareOptionalText,
    ),
    capabilities: uniqueReportedValues(
      plugins.flatMap((plugin) => plugin.requested_permissions),
    ).sort(compareOptionalText),
  };
}

export function filterForgePlugins(
  plugins: readonly PluginView[],
  filters: ForgeFilters,
): PluginView[] {
  const result = plugins.filter((plugin) => {
    if (
      !matchesQuery(filters.query, [
        plugin.id,
        plugin.name,
        plugin.author,
        plugin.version,
        plugin.runtime,
        plugin.state,
        plugin.last_error,
        ...plugin.requested_permissions,
      ])
    ) {
      return false;
    }
    if (filters.state !== "all" && plugin.state !== filters.state) return false;
    const approved = allRequestedCapabilitiesGranted(plugin);
    if (filters.access === "approved" && !approved) return false;
    if (filters.access === "review" && approved) return false;
    return (
      filters.capability === null ||
      plugin.requested_permissions.includes(filters.capability)
    );
  });

  return result.sort((left, right) => {
    if (filters.sort === "state") {
      return (
        compareOptionalText(left.state, right.state) ||
        compareOptionalText(left.name, right.name)
      );
    }
    return compareOptionalText(left.name, right.name);
  });
}

export function activeForgeFilterCount(filters: ForgeFilters): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.state !== "all",
    filters.access !== "all",
    filters.capability !== null,
    filters.sort !== "name",
  ]);
}

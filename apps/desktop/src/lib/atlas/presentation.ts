import {
  compareOptionalText,
  countActiveFilters,
  matchesQuery,
} from "$lib/exploration/collection";
import type { AircraftSummary, FboSummary } from "./types";

export type AtlasSearchItem = {
  id: string;
  kind: "aircraft" | "fbo";
  label: string;
  secondary: string | null;
  airportCode: string | null;
  airportName: string | null;
  mapped: boolean;
};

export type AtlasFilters = {
  query: string;
  kind: "all" | AtlasSearchItem["kind"];
  mapping: "all" | "mapped" | "unmapped";
  sort: "label" | "kind" | "airport";
};

export const defaultAtlasFilters: AtlasFilters = {
  query: "",
  kind: "all",
  mapping: "all",
  sort: "label",
};

export function atlasSearchItems(
  aircraft: readonly AircraftSummary[],
  fbos: readonly FboSummary[],
): AtlasSearchItem[] {
  return [
    ...aircraft.map((item) => ({
      id: item.id,
      kind: "aircraft" as const,
      label: item.registration?.trim() || "Unregistered aircraft",
      secondary: item.model?.trim() || null,
      airportCode: item.current_airport?.icao?.trim() || null,
      airportName: item.current_airport?.name?.trim() || null,
      mapped: item.location !== null,
    })),
    ...fbos.map((item) => ({
      id: item.id,
      kind: "fbo" as const,
      label: item.name?.trim() || "Unnamed FBO",
      secondary: item.airport?.name?.trim() || null,
      airportCode: item.airport?.icao?.trim() || null,
      airportName: item.airport?.name?.trim() || null,
      mapped:
        item.airport?.location !== null && item.airport?.location !== undefined,
    })),
  ];
}

export function filterAtlasItems(
  items: readonly AtlasSearchItem[],
  filters: AtlasFilters,
): AtlasSearchItem[] {
  const result = items.filter((item) => {
    if (
      !matchesQuery(filters.query, [
        item.label,
        item.secondary,
        item.airportCode,
        item.airportName,
        item.kind,
      ])
    ) {
      return false;
    }
    if (filters.kind !== "all" && item.kind !== filters.kind) return false;
    if (filters.mapping === "mapped" && !item.mapped) return false;
    if (filters.mapping === "unmapped" && item.mapped) return false;
    return true;
  });

  return result.sort((left, right) => {
    if (filters.sort === "kind") {
      return (
        compareOptionalText(left.kind, right.kind) ||
        compareOptionalText(left.label, right.label)
      );
    }
    if (filters.sort === "airport") {
      return (
        compareOptionalText(left.airportCode, right.airportCode) ||
        compareOptionalText(left.label, right.label)
      );
    }
    return compareOptionalText(left.label, right.label);
  });
}

export function activeAtlasFilterCount(filters: AtlasFilters): number {
  return countActiveFilters([
    filters.query.trim().length > 0,
    filters.kind !== "all",
    filters.mapping !== "all",
    filters.sort !== "label",
  ]);
}

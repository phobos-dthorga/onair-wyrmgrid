import type { ChartSpec } from "$lib/charts/types";
import type { FleetCompositionPoint, FleetHistoryPoint } from "./types";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
});

function chartTime(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : dateFormatter.format(parsed);
}

export function fleetGrowthChart(
  points: FleetHistoryPoint[],
): ChartSpec | null {
  const observedAt = points.at(-1)?.observed_at;
  if (!observedAt || points.length === 0) return null;
  return {
    schema_version: 1,
    id: "hoard-fleet-growth",
    title: "Fleet growth",
    description:
      "Aircraft retained in each successful OnAir fleet observation.",
    kind: "area",
    category_axis_label: "Observation",
    value_axis_label: "Aircraft",
    series: [
      {
        id: "aircraft-count",
        label: "Aircraft",
        points: points.map((point) => ({
          category: chartTime(point.observed_at),
          value: point.aircraft_count,
        })),
      },
    ],
    provenance: {
      kind: "calculated",
      source: "WyrmGrid Hoard retained OnAir facts",
      observed_at: observedAt,
    },
  };
}

export function fleetCompositionChart(
  points: FleetCompositionPoint[],
  observedAt: string | undefined,
): ChartSpec | null {
  if (!observedAt || points.length === 0) return null;
  return {
    schema_version: 1,
    id: "hoard-fleet-composition",
    title: "Fleet composition",
    description:
      "Aircraft grouped by the model reported in the selected observation.",
    kind: "bar",
    category_axis_label: "Model",
    value_axis_label: "Aircraft",
    series: [
      {
        id: "model-count",
        label: "Aircraft",
        points: points.map((point) => ({
          category: point.model,
          value: point.aircraft_count,
        })),
      },
    ],
    provenance: {
      kind: "calculated",
      source: "WyrmGrid Hoard retained OnAir facts",
      observed_at: observedAt,
    },
  };
}

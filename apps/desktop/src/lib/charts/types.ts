export const CHART_SCHEMA_VERSION = 1 as const;

export type ProvenanceKind =
  | "on_air_fact"
  | "external_fact"
  | "external_calculation"
  | "calculated"
  | "recommendation";

export type ChartSpec = {
  schema_version: typeof CHART_SCHEMA_VERSION;
  id: string;
  title: string;
  description?: string;
  kind: "line" | "area" | "bar";
  category_axis_label?: string;
  value_axis_label?: string;
  unit?: string;
  series: ChartSeries[];
  provenance: {
    kind: ProvenanceKind;
    source: string;
    observed_at: string;
  };
};

export type ChartSeries = {
  id: string;
  label: string;
  points: Array<{ category: string; value: number; gap_before?: boolean }>;
};

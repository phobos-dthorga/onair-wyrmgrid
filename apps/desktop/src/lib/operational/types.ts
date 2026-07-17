export type ProvenanceKind =
  | "on_air_fact"
  | "external_fact"
  | "external_calculation"
  | "calculated"
  | "recommendation";

export type Coordinates = { latitude: number; longitude: number };

export type OperationalProvenance = {
  kind: ProvenanceKind;
  provider: string;
  provider_revision?: string;
  generated_at?: string;
  retrieved_at: string;
  transformation_version: number;
  freshness: "current" | "stale" | "unknown";
};

export type Observation<T> = {
  value: T;
  provenance: OperationalProvenance;
};

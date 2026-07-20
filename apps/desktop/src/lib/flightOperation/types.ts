export type FlightOperationStage =
  | "plan"
  | "weather"
  | "jobs"
  | "manifest"
  | "fleet"
  | "staff"
  | "review"
  | "atlas";

export type FlightOperationStageState =
  | "not_started"
  | "available"
  | "ready"
  | "needs_attention"
  | "stale"
  | "unavailable";

export type FlightOperationJourneyView = {
  schema_version: number;
  stages: Array<{
    stage: FlightOperationStage;
    state: FlightOperationStageState;
  }>;
};

export type FlightOperationContextChange =
  "none" | "plan" | "job" | "plan_and_job";

export type FlightOperationRevisionReason =
  "initial" | "plan_changed" | "job_changed" | "plan_and_job_changed";

export type ManifestUnavailableField = "passenger_count" | "freight_weight";

export type FlightOperationView = {
  schema_version: number;
  id: string;
  revision: number;
  reason: FlightOperationRevisionReason;
  operation_created_at: string;
  revision_created_at: string;
  plan_id: string;
  origin: string;
  destination: string;
  selected_job_id?: string;
  manifest: {
    schema_version: number;
    legs: Array<{
      source_job_leg_id: string;
      sequence: number;
      departure?: { icao?: string; name?: string };
      destination?: { icao?: string; name?: string };
      passengers?: { count: number };
      freight?: { weight_lb: number };
      unavailable_fields: ManifestUnavailableField[];
    }>;
  };
  aircraft_assignment?: {
    revision: number;
    reviewed_at: string;
    id: string;
    registration?: string;
    model?: string;
    evidence_observed_at: string;
  };
  fleet_reconciliation: {
    schema_version: number;
    fleet_available: boolean;
    fleet_observed_at?: string;
    candidate?: {
      id: string;
      basis: "registration" | "exact_model" | "reviewed_assignment";
      registration?: string;
      model?: string;
      current_airport_icao?: string;
    };
    assignable_aircraft: Array<{
      id: string;
      registration?: string;
      model?: string;
      current_airport_icao?: string;
    }>;
    manifest_coverage: {
      leg_count: number;
      passenger_legs_reported: number;
      freight_legs_reported: number;
      source_gaps_present: boolean;
    };
    findings: import("$lib/dispatch/types").DispatchFinding[];
    provenance: import("$lib/operational/types").OperationalProvenance;
  };
};

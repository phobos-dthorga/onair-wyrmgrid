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

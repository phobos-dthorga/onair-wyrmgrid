import type {
  FlightOperationContextChange,
  FlightOperationView,
} from "./types";

export type ManifestHandoffState =
  "empty" | "staged_initial" | "staged_revision" | "attached" | "retained";

export function manifestHandoffState(
  operation: FlightOperationView | undefined,
  operationChange: FlightOperationContextChange,
  selectedJobId: string | undefined,
): ManifestHandoffState {
  if (selectedJobId) {
    if (!operation) return "staged_initial";
    if (
      operationChange === "job" ||
      operationChange === "plan_and_job" ||
      operation.selected_job_id !== selectedJobId
    ) {
      return "staged_revision";
    }
    return "attached";
  }
  return operation?.selected_job_id ? "retained" : "empty";
}

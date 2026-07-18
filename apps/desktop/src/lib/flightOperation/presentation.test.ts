import { describe, expect, it } from "vitest";
import type { FlightOperationView } from "./types";
import { manifestHandoffState } from "./presentation";

const acceptedOperation = {
  selected_job_id: "job-a",
} as FlightOperationView;

describe("flight-operation manifest handoff presentation", () => {
  it("stages selected evidence for an explicit initial revision", () => {
    expect(manifestHandoffState(undefined, "none", "job-a")).toBe(
      "staged_initial",
    );
  });

  it("does not imply changed job evidence rewrote the accepted revision", () => {
    expect(manifestHandoffState(acceptedOperation, "job", "job-a")).toBe(
      "staged_revision",
    );
    expect(
      manifestHandoffState(acceptedOperation, "plan_and_job", "job-b"),
    ).toBe("staged_revision");
  });

  it("distinguishes current and retained accepted evidence", () => {
    expect(manifestHandoffState(acceptedOperation, "none", "job-a")).toBe(
      "attached",
    );
    expect(manifestHandoffState(acceptedOperation, "none", undefined)).toBe(
      "retained",
    );
  });

  it("keeps an operation without job evidence manifest-empty", () => {
    expect(
      manifestHandoffState(
        { ...acceptedOperation, selected_job_id: undefined },
        "none",
        undefined,
      ),
    ).toBe("empty");
  });
});

import { describe, expect, it } from "vitest";
import {
  DesktopOperationFailure,
  operationErrorMessage,
  type OperationError,
} from "./client";

function failure(code: string, message: string): DesktopOperationFailure {
  const operation: OperationError = {
    code,
    message,
    retryable: true,
    reportable: false,
  };
  return new DesktopOperationFailure(operation);
}

describe("desktop operation error presentation", () => {
  it("uses an explicitly registered catalogue message", () => {
    expect(
      operationErrorMessage(
        failure("onair.rate_limited", "Provider fallback"),
        "Caller fallback",
      ),
    ).toContain("OnAir is rate-limiting requests");
  });

  it("preserves the controlled operation message for an unmapped code", () => {
    expect(
      operationErrorMessage(
        failure("desktop.command_failed", "Controlled desktop failure"),
        "Caller fallback",
      ),
    ).toBe("Controlled desktop failure");
  });

  it("uses the caller fallback for non-operation failures", () => {
    expect(operationErrorMessage(new Error("unknown"), "Caller fallback")).toBe(
      "Caller fallback",
    );
  });
});

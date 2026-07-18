import { describe, expect, it } from "vitest";
import { dispatchPreviewReady } from "./sample";
import { dispatchFindingMessageKeys } from "./localization";

describe("dispatch finding localization", () => {
  it("maps every preview finding without constructing catalogue keys", () => {
    for (const finding of dispatchPreviewReady.comparison?.findings ?? []) {
      expect(dispatchFindingMessageKeys[finding.message_key]).toBeDefined();
    }
  });
});

import { describe, expect, it } from "vitest";
import {
  capabilityTranslationKey,
  lifetimeTranslationKey,
} from "./presentation";

describe("authorization presentation", () => {
  it("keeps capability labels shared without hiding unknown capabilities", () => {
    expect(capabilityTranslationKey("plugin_storage")).toBe(
      "security-capability-plugin-storage",
    );
    expect(capabilityTranslationKey("future_capability")).toBeNull();
  });

  it("uses the shared lifetime catalogue", () => {
    const testCases = [
      ["once", "security-lifetime-once"],
      ["session", "security-lifetime-session"],
      ["standing", "security-lifetime-standing"],
    ] as const;

    for (const [input, expected] of testCases) {
      expect(lifetimeTranslationKey(input)).toBe(expected);
    }
  });
});

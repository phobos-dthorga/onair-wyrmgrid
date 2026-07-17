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
    expect(lifetimeTranslationKey("session")).toBe("security-lifetime-session");
  });
});

import { describe, expect, it } from "vitest";
import {
  pluginSettingChoiceKey,
  pluginSettingPresentation,
} from "./configuration";

describe("host-owned plugin configuration presentation", () => {
  it("maps supported host setting keys to fixed translation keys", () => {
    expect(pluginSettingPresentation("forecast_refresh_minutes")).toEqual({
      label: "forge-setting-forecast-refresh",
      detail: "forge-setting-forecast-refresh-detail",
    });
    expect(pluginSettingPresentation("radar_refresh_minutes").label).toBe(
      "forge-setting-radar-refresh",
    );
  });

  it("degrades safely when a newer host returns an unknown choice", () => {
    expect(pluginSettingPresentation("unexpected").label).toBe(
      "forge-setting-unknown",
    );
    expect(pluginSettingChoiceKey("7")).toBe("forge-setting-refresh-unknown");
  });
});

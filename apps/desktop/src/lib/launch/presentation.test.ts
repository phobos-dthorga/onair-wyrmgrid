import { describe, expect, it } from "vitest";
import {
  launchArtworkTone,
  MINIMUM_LAUNCH_DISPLAY_MS,
  remainingLaunchDisplayTime,
} from "./presentation";

describe("launch artwork presentation", () => {
  it("selects the brighter artwork only for a light theme canvas", () => {
    expect(launchArtworkTone("#07110F")).toBe("dark");
    expect(launchArtworkTone("#100708")).toBe("dark");
    expect(launchArtworkTone("#000000")).toBe("dark");
    expect(launchArtworkTone("#F4F0E7")).toBe("light");
  });

  it("falls back safely when a canvas colour is malformed", () => {
    expect(launchArtworkTone("transparent")).toBe("dark");
    expect(launchArtworkTone("#fff")).toBe("dark");
  });

  it("holds only the remainder of the minimum launch interval", () => {
    expect(remainingLaunchDisplayTime(1_000, 1_250)).toBe(
      MINIMUM_LAUNCH_DISPLAY_MS - 250,
    );
    expect(remainingLaunchDisplayTime(1_000, 2_000)).toBe(0);
    expect(remainingLaunchDisplayTime(1_000, 900)).toBe(
      MINIMUM_LAUNCH_DISPLAY_MS,
    );
  });
});

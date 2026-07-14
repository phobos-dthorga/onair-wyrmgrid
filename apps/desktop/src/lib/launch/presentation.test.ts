import { describe, expect, it } from "vitest";
import {
  launchArtworkTone,
  MINIMUM_LAUNCH_DISPLAY_MS,
  remainingLaunchDisplayTime,
  shouldRenderLaunchArtwork,
  viewportPresentation,
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

  it("never mounts artwork before startup options or when it is disabled", () => {
    expect(shouldRenderLaunchArtwork(false, false)).toBe(false);
    expect(shouldRenderLaunchArtwork(true, true)).toBe(false);
    expect(shouldRenderLaunchArtwork(true, false)).toBe(true);
  });

  it("classifies representative laptop and handheld viewports", () => {
    expect(viewportPresentation(1366, 768)).toBe("standard");
    expect(viewportPresentation(1280, 720)).toBe("short");
    expect(viewportPresentation(800, 1280)).toBe("narrow");
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

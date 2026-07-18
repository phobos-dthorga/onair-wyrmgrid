import { describe, expect, it } from "vitest";
import type { Translator } from "$lib/i18n/runtime";
import { presentWeatherRendererStatus } from "./statusPresentation";

const translate = ((key, arguments_) =>
  `${key}${arguments_ ? ` ${JSON.stringify(arguments_)}` : ""}`) as Translator;

describe("weather renderer status presentation", () => {
  it("presents an initializing renderer without hiding the MapLibre fallback", () => {
    const status = presentWeatherRendererStatus(
      {
        profile: "enhanced",
        rendererStatus: { state: "initializing" },
        lowResource: false,
        reducedMotion: false,
        stationCount: 2,
        windCount: 1,
        effectCount: 1,
      },
      translate,
    );

    expect(status.title).toBe("atlas-weather-status-title-starting");
    expect(status.detail).toBe("atlas-weather-status-detail-starting");
    expect(status.stationCount).toContain('"count":2');
  });

  it("uses explicit quality and count messages for an adapted renderer", () => {
    const status = presentWeatherRendererStatus(
      {
        profile: "cinematic",
        rendererStatus: {
          state: "ready",
          backend: "webgpu",
          quality: "balanced",
        },
        lowResource: false,
        reducedMotion: false,
        stationCount: 3,
        windCount: 2,
        effectCount: 4,
      },
      translate,
    );

    expect(status.title).toBe("atlas-weather-status-title-webgpu");
    expect(status.detail).toContain("atlas-weather-status-detail-adaptive");
    expect(status.detail).toContain("atlas-weather-quality-balanced");
  });

  it("prioritizes low-resource and reduced-motion explanations", () => {
    expect(
      presentWeatherRendererStatus(
        {
          profile: "compatibility",
          rendererStatus: { state: "disabled" },
          lowResource: true,
          reducedMotion: false,
          stationCount: 1,
          windCount: 0,
          effectCount: 0,
        },
        translate,
      ).detail,
    ).toBe("atlas-weather-status-detail-low-resource");

    expect(
      presentWeatherRendererStatus(
        {
          profile: "enhanced",
          rendererStatus: {
            state: "ready",
            backend: "webgl2",
            quality: "full",
          },
          lowResource: false,
          reducedMotion: true,
          stationCount: 1,
          windCount: 3,
          effectCount: 2,
        },
        translate,
      ).detail,
    ).toContain("atlas-weather-status-detail-reduced-motion");
  });
});

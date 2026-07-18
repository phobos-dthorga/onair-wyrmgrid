import type { Translator } from "$lib/i18n/runtime";
import type { WeatherRenderingProfile } from "$lib/settings/types";
import type { AdaptiveWeatherQuality } from "./quality";
import type { WeatherRendererStatus } from "./types";

export type WeatherRendererStatusPresentation = {
  title: string;
  stationCount: string;
  detail: string;
  sourceBoundary: string;
};

export type WeatherRendererStatusPresentationInput = {
  profile: WeatherRenderingProfile;
  rendererStatus: WeatherRendererStatus;
  lowResource: boolean;
  reducedMotion: boolean;
  stationCount: number;
  windCount: number;
  effectCount: number;
};

const adaptiveQualityMessageKeys = {
  balanced: "atlas-weather-quality-balanced",
  minimum: "atlas-weather-quality-minimum",
} as const satisfies Record<
  Exclude<AdaptiveWeatherQuality, "full">,
  Parameters<Translator>[0]
>;

export function presentWeatherRendererStatus(
  input: WeatherRendererStatusPresentationInput,
  translate: Translator,
): WeatherRendererStatusPresentation {
  return {
    title: presentTitle(input, translate),
    stationCount: translate("atlas-weather-status-stations", {
      count: input.stationCount,
    }),
    detail: presentDetail(input, translate),
    sourceBoundary: translate("atlas-weather-status-source-boundary"),
  };
}

function presentTitle(
  input: WeatherRendererStatusPresentationInput,
  translate: Translator,
): string {
  if (input.profile === "compatibility") {
    return translate("atlas-weather-status-title-fallback");
  }
  switch (input.rendererStatus.state) {
    case "ready":
      return translate(
        input.rendererStatus.backend === "webgpu"
          ? "atlas-weather-status-title-webgpu"
          : "atlas-weather-status-title-webgl2",
      );
    case "initializing":
      return translate("atlas-weather-status-title-starting");
    case "device_lost":
      return translate("atlas-weather-status-title-device-lost");
    case "unavailable":
      return translate("atlas-weather-status-title-unavailable");
    case "disabled":
      return translate(
        input.profile === "cinematic"
          ? "atlas-weather-status-title-cinematic"
          : "atlas-weather-status-title-enhanced",
      );
  }
}

function presentDetail(
  input: WeatherRendererStatusPresentationInput,
  translate: Translator,
): string {
  if (input.lowResource) {
    return translate("atlas-weather-status-detail-low-resource");
  }
  if (input.profile === "compatibility") {
    return translate("atlas-weather-status-detail-compatibility");
  }
  if (input.rendererStatus.state === "initializing") {
    return translate("atlas-weather-status-detail-starting");
  }
  if (
    input.rendererStatus.state === "unavailable" ||
    input.rendererStatus.state === "device_lost"
  ) {
    return translate("atlas-weather-status-detail-fallback");
  }
  if (input.reducedMotion) {
    return translate("atlas-weather-status-detail-reduced-motion", {
      winds: input.windCount,
    });
  }
  if (
    input.rendererStatus.state === "ready" &&
    input.rendererStatus.quality !== "full"
  ) {
    return translate("atlas-weather-status-detail-adaptive", {
      winds: input.windCount,
      cells: input.effectCount,
      quality: translate(
        adaptiveQualityMessageKeys[input.rendererStatus.quality],
      ),
    });
  }
  return translate("atlas-weather-status-detail-active", {
    winds: input.windCount,
    cells: input.effectCount,
  });
}

import type { WeatherGraphicsPolicy } from "$lib/weather/graphics";
import type { WeatherRenderingProfile } from "$lib/settings/types";
import type { WeatherRendererUpdate, WeatherScreenPoint } from "./types";
import type {
  WeatherRenderCell,
  WeatherRenderEffect,
} from "./weatherRenderScene";

export type WeatherGalleryEffect = "all" | WeatherRenderEffect;

const GALLERY_CELLS: readonly WeatherRenderCell[] = [
  {
    id: "gallery-cloud",
    source: "plugin_grid",
    longitude: -112,
    latitude: 28,
    effect: "cloud",
    intensity: 0.62,
    windBearing: 245,
    windSpeedKt: 16,
  },
  {
    id: "gallery-rain",
    source: "plugin_grid",
    longitude: -38,
    latitude: 28,
    effect: "rain",
    intensity: 0.78,
    windBearing: 220,
    windSpeedKt: 24,
  },
  {
    id: "gallery-convective",
    source: "plugin_grid",
    longitude: 38,
    latitude: 28,
    effect: "convective",
    intensity: 1,
    windBearing: 185,
    windSpeedKt: 36,
  },
  {
    id: "gallery-snow",
    source: "plugin_grid",
    longitude: 112,
    latitude: 28,
    effect: "snow",
    intensity: 0.72,
    windBearing: 285,
    windSpeedKt: 18,
  },
  {
    id: "gallery-obscuration",
    source: "plugin_grid",
    longitude: -55,
    latitude: -34,
    effect: "obscuration",
    intensity: 0.58,
    windBearing: 260,
    windSpeedKt: 11,
  },
  {
    id: "gallery-dust",
    source: "plugin_grid",
    longitude: 55,
    latitude: -34,
    effect: "dust",
    intensity: 0.82,
    windBearing: 250,
    windSpeedKt: 31,
  },
];

function galleryPolicy(
  profile: Exclude<WeatherRenderingProfile, "compatibility">,
  animation: boolean,
): WeatherGraphicsPolicy {
  const cinematic = profile === "cinematic";
  return {
    profile,
    atmosphere: true,
    clouds: true,
    precipitation: true,
    lightning: true,
    dust: true,
    animation,
    lightningFlashes: false,
    particleScale: cinematic ? 1 : 0.58,
    frameRate: cinematic ? 30 : 20,
  };
}

export function buildWeatherGalleryUpdate(
  profile: Exclude<WeatherRenderingProfile, "compatibility">,
  effect: WeatherGalleryEffect,
  animation: boolean,
): WeatherRendererUpdate {
  const selectedCells = GALLERY_CELLS.filter(
    (cell) => effect === "all" || cell.effect === effect,
  );
  const cells =
    effect === "all"
      ? [...selectedCells]
      : selectedCells.map((cell) => ({
          ...cell,
          longitude: 0,
          latitude: 0,
        }));
  return {
    scene: {
      signature: `weather-gallery:${effect}:${cells.map((cell) => cell.id).join(",")}`,
      cells,
    },
    policy: galleryPolicy(profile, animation),
  };
}

export function projectWeatherGalleryPoint(
  longitude: number,
  latitude: number,
  width: number,
  height: number,
): WeatherScreenPoint {
  return {
    x: width / 2 + (longitude / 300) * width,
    y: height / 2 - (latitude / 130) * height,
    surfaceVisibility: 1,
  };
}

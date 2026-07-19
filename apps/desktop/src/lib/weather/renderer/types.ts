import type { WeatherGraphicsPolicy } from "$lib/weather/graphics";
import type { WeatherRenderScene } from "./weatherRenderScene";
import type { AdaptiveWeatherQuality } from "./quality";

export type WeatherRendererBackend = "webgpu" | "webgl2";

export type WeatherRendererStatus =
  | { state: "disabled" }
  | { state: "initializing" }
  | {
      state: "ready";
      backend: WeatherRendererBackend;
      quality: AdaptiveWeatherQuality;
    }
  | { state: "unavailable"; reason: string }
  | { state: "device_lost"; backend: WeatherRendererBackend; reason: string };

export type WeatherScreenPoint = {
  x: number;
  y: number;
  surfaceVisibility: number;
};

export type WeatherRendererFrame = {
  width: number;
  height: number;
  pixelRatio: number;
  zoom: number;
  bearing: number;
  projectionKey: string;
  timeMs: number;
  project: (longitude: number, latitude: number) => WeatherScreenPoint;
  surfaceVisibilityAt: (x: number, y: number) => number;
};

export type WeatherRendererUpdate = {
  scene: WeatherRenderScene;
  policy: WeatherGraphicsPolicy;
};

export interface WeatherRenderer {
  readonly backend: WeatherRendererBackend;
  readonly quality: AdaptiveWeatherQuality;
  update(update: WeatherRendererUpdate): void;
  render(frame: WeatherRendererFrame): void;
  dispose(): void;
}

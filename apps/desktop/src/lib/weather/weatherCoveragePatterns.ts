import type { ExpressionSpecification } from "maplibre-gl";
import type { WeatherEffect } from "./atlasWeather";

export type WeatherZoneKind =
  Exclude<WeatherEffect, "none"> | "cloud" | "radar";
export type WeatherZonePatternRole = "fill" | "marker";

export type WeatherZonePatternImage = {
  width: number;
  height: number;
  data: Uint8ClampedArray;
};

export const WEATHER_ZONE_KINDS: readonly WeatherZoneKind[] = [
  "cloud",
  "rain",
  "snow",
  "convective",
  "obscuration",
  "dust",
  "radar",
];

export const WEATHER_ZONE_COLORS: Readonly<Record<WeatherZoneKind, string>> = {
  cloud: "#8fa8bb",
  rain: "#419fd1",
  snow: "#d7edf4",
  convective: "#d46175",
  obscuration: "#a491b4",
  dust: "#c2874f",
  radar: "#6ec5df",
};

const FILL_PATTERN_SIZE = 64;
const MARKER_PATTERN_SIZE = 128;

function positiveModulo(value: number, divisor: number): number {
  return ((value % divisor) + divisor) % divisor;
}

function motifPixel(kind: WeatherZoneKind, x: number, y: number): boolean {
  // Every motif period divides the 64px fill tile so opposing edges repeat cleanly.
  switch (kind) {
    case "cloud": {
      const wave = Math.round(4 + Math.sin((x / 16) * Math.PI) * 2);
      return Math.abs(positiveModulo(y, 16) - wave) <= 1;
    }
    case "rain":
      return positiveModulo(x + y, 16) <= 2;
    case "snow": {
      const localX = positiveModulo(x, 16) - 8;
      const localY = positiveModulo(y, 16) - 8;
      return (
        (Math.abs(localX) <= 1 && Math.abs(localY) <= 4) ||
        (Math.abs(localY) <= 1 && Math.abs(localX) <= 4) ||
        (Math.abs(Math.abs(localX) - Math.abs(localY)) <= 1 &&
          Math.abs(localX) <= 3)
      );
    }
    case "convective": {
      const localX = positiveModulo(x, 16);
      const localY = positiveModulo(y, 16);
      const zigzag = localX <= 8 ? localX / 2 : (16 - localX) / 2;
      return Math.abs(localY - (5 + zigzag)) <= 1;
    }
    case "obscuration": {
      const row = Math.floor(y / 8);
      const localX = positiveModulo(x + (row % 2) * 4, 8) - 4;
      const localY = positiveModulo(y, 8) - 4;
      return localX * localX + localY * localY <= 3;
    }
    case "dust":
      return positiveModulo(x + y, 16) <= 1 || positiveModulo(x - y, 16) <= 1;
    case "radar":
      return (
        (positiveModulo(x, 16) <= 1 && positiveModulo(y, 8) <= 5) ||
        (positiveModulo(y, 16) <= 1 && positiveModulo(x, 8) <= 5)
      );
  }
}

function parseHexColor(value: string): [number, number, number] {
  return [
    Number.parseInt(value.slice(1, 3), 16),
    Number.parseInt(value.slice(3, 5), 16),
    Number.parseInt(value.slice(5, 7), 16),
  ];
}

function writePixel(
  data: Uint8ClampedArray,
  size: number,
  x: number,
  y: number,
  color: [number, number, number],
  alpha: number,
): void {
  const offset = (y * size + x) * 4;
  data[offset] = color[0];
  data[offset + 1] = color[1];
  data[offset + 2] = color[2];
  data[offset + 3] = Math.max(data[offset + 3], alpha);
}

export function weatherZonePatternId(
  kind: WeatherZoneKind,
  role: WeatherZonePatternRole,
): string {
  return `wyrmgrid-weather-zone-${role}-${kind}`;
}

export function weatherZonePatternExpression(
  property: "condition" | "effect",
  role: WeatherZonePatternRole,
): ExpressionSpecification {
  return [
    "match",
    ["get", property],
    ...WEATHER_ZONE_KINDS.flatMap((kind) => [
      kind,
      weatherZonePatternId(kind, role),
    ]),
    weatherZonePatternId("cloud", role),
  ] as unknown as ExpressionSpecification;
}

export function createWeatherZonePatternImage(
  kind: WeatherZoneKind,
  role: WeatherZonePatternRole,
): WeatherZonePatternImage {
  const size = role === "fill" ? FILL_PATTERN_SIZE : MARKER_PATTERN_SIZE;
  const data = new Uint8ClampedArray(size * size * 4);
  const color = parseHexColor(WEATHER_ZONE_COLORS[kind]);
  const markerRadius = size / 2 - 5;
  const markerRadiusSquared = markerRadius * markerRadius;
  const patternScale = role === "marker" ? 2 : 1;

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const fromCentreX = x + 0.5 - size / 2;
      const fromCentreY = y + 0.5 - size / 2;
      const distanceSquared = fromCentreX ** 2 + fromCentreY ** 2;
      if (role === "marker" && distanceSquared > markerRadiusSquared) continue;

      if (role === "marker") {
        writePixel(data, size, x, y, color, 24);
        if (
          distanceSquared >= (markerRadius - 2.5) ** 2 &&
          distanceSquared <= markerRadiusSquared
        ) {
          writePixel(data, size, x, y, color, 176);
        }
      }
      if (
        motifPixel(
          kind,
          Math.floor(x / patternScale),
          Math.floor(y / patternScale),
        )
      ) {
        writePixel(data, size, x, y, color, role === "fill" ? 196 : 184);
      }
    }
  }

  return { width: size, height: size, data };
}

export function weatherZonePatternImages(): Array<{
  id: string;
  image: WeatherZonePatternImage;
  role: WeatherZonePatternRole;
}> {
  return WEATHER_ZONE_KINDS.flatMap((kind) =>
    (["fill", "marker"] as const).map((role) => ({
      id: weatherZonePatternId(kind, role),
      image: createWeatherZonePatternImage(kind, role),
      role,
    })),
  );
}

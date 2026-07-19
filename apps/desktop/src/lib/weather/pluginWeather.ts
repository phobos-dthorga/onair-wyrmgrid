import type {
  GlobalWeatherGridPoint,
  GlobalWeatherLayer,
  GlobalWeatherRasterTile,
  PublishedPluginWeatherLayer,
} from "$lib/forge/types";

export type PluginWeatherGridFeatureCollection = {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    geometry: { type: "Point"; coordinates: [number, number] };
    properties: {
      id: string;
      plugin_id: string;
      layer_title: string;
      condition: string;
      temperature_c: number | null;
      precipitation_mm: number | null;
      cloud_cover_percent: number | null;
      wind_direction_degrees: number | null;
      wind_speed_kt: number | null;
    };
  }>;
};

export type PluginRadarTile = {
  id: string;
  url: string;
  coverage_url?: string;
  coordinates: [
    [number, number],
    [number, number],
    [number, number],
    [number, number],
  ];
};

export type PluginRadarFrame = {
  id: string;
  frame_time: string;
  retrieved_at: string;
  freshness: "current" | "stale" | "unknown";
  tiles: PluginRadarTile[];
};

export type PluginRadarTimeline = {
  id: string;
  plugin_id: string;
  layer_id: string;
  layer_title: string;
  provider: string;
  frames: PluginRadarFrame[];
};

export function pluginWeatherGridFeatures(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): PluginWeatherGridFeatureCollection {
  return {
    type: "FeatureCollection",
    features: publishedLayers.flatMap((published) =>
      published.layer.data.kind === "grid"
        ? displayedGlobalWeatherGridPoints(published.layer).map((point) => ({
            type: "Feature" as const,
            geometry: {
              type: "Point" as const,
              coordinates: [
                point.location.longitude,
                point.location.latitude,
              ] as [number, number],
            },
            properties: {
              id: `${published.plugin_id}:${published.layer.id}:${point.id}`,
              plugin_id: published.plugin_id,
              layer_title: published.layer.title,
              condition: point.condition,
              temperature_c: point.temperature_c ?? null,
              precipitation_mm: point.precipitation_mm ?? null,
              cloud_cover_percent: point.cloud_cover_percent ?? null,
              wind_direction_degrees: point.wind_direction_degrees ?? null,
              wind_speed_kt: point.wind_speed_kt ?? null,
            },
          }))
        : [],
    ),
  };
}

export function pluginRadarTimelines(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): PluginRadarTimeline[] {
  const timelines = new Map<string, PluginRadarTimeline>();
  for (const published of publishedLayers) {
    const data = published.layer.data;
    if (data.kind !== "raster_tiles") continue;
    const timelineId = `${published.plugin_id}:${published.layer.id}`;
    const timeline = timelines.get(timelineId) ?? {
      id: timelineId,
      plugin_id: published.plugin_id,
      layer_id: published.layer.id,
      layer_title: published.layer.title,
      provider: published.layer.provenance.provider,
      frames: [],
    };
    const frame: PluginRadarFrame = {
      id: `${timelineId}:${data.frame_time}`,
      frame_time: data.frame_time,
      retrieved_at: published.layer.provenance.retrieved_at,
      freshness: published.layer.provenance.freshness,
      tiles: data.tiles.map((tile) => ({
        id: `${timelineId}:${tile.zoom}-${tile.x}-${tile.y}`,
        url: `data:image/png;base64,${tile.png_base64}`,
        coverage_url: tile.coverage_png_base64
          ? `data:image/png;base64,${tile.coverage_png_base64}`
          : undefined,
        coordinates: rasterTileCoordinates(tile),
      })),
    };
    const previous = timeline.frames.findIndex(
      (candidate) => candidate.frame_time === frame.frame_time,
    );
    if (previous >= 0) timeline.frames[previous] = frame;
    else timeline.frames.push(frame);
    timelines.set(timelineId, timeline);
  }
  return [...timelines.values()]
    .map((timeline) => ({
      ...timeline,
      frames: timeline.frames.sort(
        (left, right) =>
          Date.parse(left.frame_time) - Date.parse(right.frame_time),
      ),
    }))
    .sort((left, right) => left.id.localeCompare(right.id));
}

export function selectedRadarFrames(
  timelines: readonly PluginRadarTimeline[],
  selectedIndex: number,
  staticPresentation: boolean,
): PluginRadarFrame[] {
  return timelines.flatMap((timeline) => {
    if (timeline.frames.length === 0) return [];
    const index = staticPresentation
      ? timeline.frames.length - 1
      : Math.max(0, Math.min(selectedIndex, timeline.frames.length - 1));
    return [timeline.frames[index]];
  });
}

export function longestRadarTimeline(
  timelines: readonly PluginRadarTimeline[],
): PluginRadarTimeline | undefined {
  return timelines.reduce<PluginRadarTimeline | undefined>(
    (longest, timeline) =>
      !longest || timeline.frames.length > longest.frames.length
        ? timeline
        : longest,
    undefined,
  );
}

export function pluginWeatherItemCount(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): number {
  const gridPoints = publishedLayers.reduce(
    (total, published) =>
      total +
      (published.layer.data.kind === "grid"
        ? displayedGlobalWeatherGridPoints(published.layer).length
        : 0),
    0,
  );
  const currentRadarTiles = pluginRadarTimelines(publishedLayers).reduce(
    (total, timeline) => total + (timeline.frames.at(-1)?.tiles.length ?? 0),
    0,
  );
  return gridPoints + currentRadarTiles;
}

/**
 * Selects one presentation-time sample per model-grid location. The complete
 * temporal grid remains available to Rust for route analysis; Atlas should not
 * draw six overlapping weather volumes and support cells at every location.
 */
export function displayedGlobalWeatherGridPoints(
  layer: GlobalWeatherLayer,
): GlobalWeatherGridPoint[] {
  if (layer.data.kind !== "grid") return [];
  const timedByLocation = new Map<string, GlobalWeatherGridPoint>();
  const legacy: GlobalWeatherGridPoint[] = [];
  const reference = Date.parse(layer.provenance.retrieved_at);
  for (const point of layer.data.points) {
    if (!point.valid_at) {
      legacy.push(point);
      continue;
    }
    const key = `${point.location.latitude}:${point.location.longitude}`;
    const current = timedByLocation.get(key);
    if (
      !current ||
      timeDistance(point, reference) < timeDistance(current, reference) ||
      (timeDistance(point, reference) === timeDistance(current, reference) &&
        point.id.localeCompare(current.id) < 0)
    ) {
      timedByLocation.set(key, point);
    }
  }
  if (timedByLocation.size === 0) return legacy;
  const timedLocations = new Set(timedByLocation.keys());
  return [
    ...timedByLocation.values(),
    ...legacy.filter(
      (point) =>
        !timedLocations.has(
          `${point.location.latitude}:${point.location.longitude}`,
        ),
    ),
  ];
}

function timeDistance(
  point: GlobalWeatherGridPoint,
  reference: number,
): number {
  const validAt = Date.parse(point.valid_at ?? "");
  return Number.isFinite(validAt) && Number.isFinite(reference)
    ? Math.abs(validAt - reference)
    : Number.POSITIVE_INFINITY;
}

export function rasterTileCoordinates(
  tile: Pick<GlobalWeatherRasterTile, "zoom" | "x" | "y">,
): PluginRadarTile["coordinates"] {
  return [
    webMercatorCorner(tile.zoom, tile.x, tile.y),
    webMercatorCorner(tile.zoom, tile.x + 1, tile.y),
    webMercatorCorner(tile.zoom, tile.x + 1, tile.y + 1),
    webMercatorCorner(tile.zoom, tile.x, tile.y + 1),
  ];
}

function webMercatorCorner(
  zoom: number,
  x: number,
  y: number,
): [number, number] {
  const scale = 2 ** zoom;
  const longitude = (x / scale) * 360 - 180;
  const latitude =
    (Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / scale))) * 180) / Math.PI;
  return [longitude, latitude];
}

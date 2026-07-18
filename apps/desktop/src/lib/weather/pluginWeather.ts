import type {
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

export type PluginRadarFrame = {
  id: string;
  plugin_id: string;
  layer_title: string;
  frame_time: string;
  url: string;
  coordinates: [
    [number, number],
    [number, number],
    [number, number],
    [number, number],
  ];
};

export function pluginWeatherGridFeatures(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): PluginWeatherGridFeatureCollection {
  return {
    type: "FeatureCollection",
    features: publishedLayers.flatMap((published) =>
      published.layer.data.kind === "grid"
        ? published.layer.data.points.map((point) => ({
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

export function pluginRadarFrames(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): PluginRadarFrame[] {
  return publishedLayers.flatMap((published) => {
    const data = published.layer.data;
    if (data.kind !== "raster_tiles") return [];
    return data.tiles.map((tile) => ({
      id: `${published.plugin_id}:${published.layer.id}:${tile.zoom}-${tile.x}-${tile.y}`,
      plugin_id: published.plugin_id,
      layer_title: published.layer.title,
      frame_time: data.frame_time,
      url: `data:image/png;base64,${tile.png_base64}`,
      coordinates: rasterTileCoordinates(tile),
    }));
  });
}

export function pluginWeatherItemCount(
  publishedLayers: readonly PublishedPluginWeatherLayer[],
): number {
  return publishedLayers.reduce(
    (total, published) =>
      total +
      (published.layer.data.kind === "grid"
        ? published.layer.data.points.length
        : published.layer.data.tiles.length),
    0,
  );
}

export function rasterTileCoordinates(
  tile: Pick<GlobalWeatherRasterTile, "zoom" | "x" | "y">,
): PluginRadarFrame["coordinates"] {
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

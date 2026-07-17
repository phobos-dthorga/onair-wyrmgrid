import type { AtlasAdministrativeRegion } from "./types";

export const ATLAS_ADMIN1_DATASET_URL =
  "/data/atlas/admin1-natural-earth-5.1.2.geojson";

export type AdministrativeRegionLabelBand = Readonly<{
  id: string;
  min_zoom: number;
  max_zoom: number;
  maximum_source_min_zoom: number;
  text_padding: number;
}>;

/**
 * Natural Earth assigns every regional label a minimum useful zoom. MapLibre
 * cannot use the camera zoom directly inside a layer filter, so Atlas divides
 * the source into exclusive bands. Only one band participates in collision
 * detection at a time; otherwise thousands of transparent labels would still
 * crowd out the names that are meant to be visible.
 */
export const ADMINISTRATIVE_REGION_LABEL_BANDS = [
  {
    id: "overview",
    min_zoom: 2.5,
    max_zoom: 3.25,
    maximum_source_min_zoom: 2,
    text_padding: 10,
  },
  {
    id: "continental",
    min_zoom: 3.25,
    max_zoom: 4.25,
    maximum_source_min_zoom: 3,
    text_padding: 8,
  },
  {
    id: "country",
    min_zoom: 4.25,
    max_zoom: 5.5,
    maximum_source_min_zoom: 4.7,
    text_padding: 7,
  },
  {
    id: "regional",
    min_zoom: 5.5,
    max_zoom: 6.75,
    maximum_source_min_zoom: 6.7,
    text_padding: 6,
  },
  {
    id: "local",
    min_zoom: 6.75,
    max_zoom: 8.25,
    maximum_source_min_zoom: 7.7,
    text_padding: 5,
  },
  {
    id: "detailed",
    min_zoom: 8.25,
    max_zoom: 10,
    maximum_source_min_zoom: 9,
    text_padding: 4,
  },
  {
    id: "close",
    min_zoom: 10,
    max_zoom: 18,
    maximum_source_min_zoom: 11,
    text_padding: 3,
  },
  {
    id: "survey",
    min_zoom: 18,
    max_zoom: 24,
    maximum_source_min_zoom: 18,
    text_padding: 2,
  },
] as const satisfies readonly AdministrativeRegionLabelBand[];

export function administrativeRegionLabelBandForZoom(
  zoom: number,
): AdministrativeRegionLabelBand | undefined {
  return ADMINISTRATIVE_REGION_LABEL_BANDS.find(
    (band) => zoom >= band.min_zoom && zoom < band.max_zoom,
  );
}

type MapRegionFeature = {
  id?: string | number;
  properties?: Record<string, unknown> | null;
};

function requiredString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function optionalString(value: unknown): string | undefined {
  return requiredString(value);
}

export function administrativeRegionFromMapFeature(
  feature: MapRegionFeature | undefined,
): AtlasAdministrativeRegion | undefined {
  const properties = feature?.properties;
  const id = requiredString(properties?.region_id);
  const name = requiredString(properties?.name);
  const level = properties?.level;
  const source = requiredString(properties?.source);
  const sourceVersion = requiredString(properties?.source_version);
  if (
    feature?.id === undefined ||
    !id ||
    !name ||
    (level !== "ADM1" && level !== "ADM2") ||
    !source ||
    !sourceVersion
  ) {
    return undefined;
  }

  return {
    id,
    feature_id: feature.id,
    level,
    name,
    name_local: optionalString(properties?.name_local),
    local_type: optionalString(properties?.local_type),
    country_name: optionalString(properties?.country_name),
    country_code: optionalString(properties?.country_code),
    subdivision_code: optionalString(properties?.subdivision_code),
    source,
    source_version: sourceVersion,
  };
}

export function administrativeRegionContext(
  region: AtlasAdministrativeRegion,
): string {
  return [region.local_type ?? region.level, region.country_name]
    .filter(Boolean)
    .join(" · ");
}

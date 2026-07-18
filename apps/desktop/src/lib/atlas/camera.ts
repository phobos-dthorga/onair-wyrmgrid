export const ATLAS_HOME_CENTER: [number, number] = [0, 0];

const GLOBAL_OVERVIEW_LONGITUDE_SPAN = 120;
const GLOBAL_OVERVIEW_LATITUDE_SPAN = 70;

/**
 * Keeps the globe itself visually centred when Atlas frames observations spread
 * across much of the world. Local and regional extents remain untouched so
 * their useful camera fit is not diluted by an artificial equator point.
 */
export function balancedOverviewCoordinates(
  coordinates: readonly [number, number][],
): [number, number][] {
  const result = coordinates.map(
    ([longitude, latitude]) => [longitude, latitude] as [number, number],
  );
  if (coordinates.length < 2) return result;

  const longitudes = coordinates.map(([longitude]) => longitude);
  const latitudes = coordinates.map(([, latitude]) => latitude);
  const minimumLongitude = Math.min(...longitudes);
  const maximumLongitude = Math.max(...longitudes);
  const minimumLatitude = Math.min(...latitudes);
  const maximumLatitude = Math.max(...latitudes);
  const globalOverview =
    maximumLongitude - minimumLongitude >= GLOBAL_OVERVIEW_LONGITUDE_SPAN ||
    maximumLatitude - minimumLatitude >= GLOBAL_OVERVIEW_LATITUDE_SPAN;
  if (!globalOverview) return result;

  const latitudeExtent = Math.max(
    Math.abs(minimumLatitude),
    Math.abs(maximumLatitude),
  );
  const centreLongitude = (minimumLongitude + maximumLongitude) / 2;
  result.push(
    [centreLongitude, -latitudeExtent],
    [centreLongitude, latitudeExtent],
  );
  return result;
}

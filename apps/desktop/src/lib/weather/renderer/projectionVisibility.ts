export type GeographicPoint = {
  longitude: number;
  latitude: number;
};

const FULL_VISIBILITY_ERROR_DEGREES = 0.05;
const HIDDEN_VISIBILITY_ERROR_DEGREES = 1.5;
const DEGREES_TO_RADIANS = Math.PI / 180;
const RADIANS_TO_DEGREES = 180 / Math.PI;

function normalizedLongitudeDeltaDegrees(
  startLongitude: number,
  endLongitude: number,
): number {
  return ((endLongitude - startLongitude + 540) % 360) - 180;
}

function angularDistanceDegrees(
  expected: GeographicPoint,
  roundTrip: GeographicPoint,
): number {
  const expectedLatitude = expected.latitude * DEGREES_TO_RADIANS;
  const roundTripLatitude = roundTrip.latitude * DEGREES_TO_RADIANS;
  const latitudeDelta =
    (roundTrip.latitude - expected.latitude) * DEGREES_TO_RADIANS;
  const longitudeDelta =
    normalizedLongitudeDeltaDegrees(expected.longitude, roundTrip.longitude) *
    DEGREES_TO_RADIANS;
  const haversine =
    Math.sin(latitudeDelta / 2) ** 2 +
    Math.cos(expectedLatitude) *
      Math.cos(roundTripLatitude) *
      Math.sin(longitudeDelta / 2) ** 2;
  return (
    2 *
    Math.asin(Math.min(1, Math.sqrt(Math.max(0, haversine)))) *
    RADIANS_TO_DEGREES
  );
}

/**
 * Estimates whether a source coordinate lies on the currently visible map
 * surface by comparing it with MapLibre's project/unproject round trip.
 *
 * Globe points behind the planet and pitched-map points above the horizon are
 * unprojected onto the nearest visible surface instead of their source
 * coordinate. The angular mismatch gives the presentation layer a bounded,
 * projection-neutral fade without depending on MapLibre's internal transform.
 */
export function weatherProjectionSurfaceVisibility(
  expected: GeographicPoint,
  roundTrip: GeographicPoint,
): number {
  if (
    !Number.isFinite(expected.longitude) ||
    !Number.isFinite(expected.latitude) ||
    !Number.isFinite(roundTrip.longitude) ||
    !Number.isFinite(roundTrip.latitude) ||
    Math.abs(expected.latitude) > 90 ||
    Math.abs(roundTrip.latitude) > 90
  ) {
    return 0;
  }

  const error = angularDistanceDegrees(expected, roundTrip);
  if (error <= FULL_VISIBILITY_ERROR_DEGREES) return 1;
  if (error >= HIDDEN_VISIBILITY_ERROR_DEGREES) return 0;

  const progress =
    (error - FULL_VISIBILITY_ERROR_DEGREES) /
    (HIDDEN_VISIBILITY_ERROR_DEGREES - FULL_VISIBILITY_ERROR_DEGREES);
  const smoothProgress = progress * progress * (3 - 2 * progress);
  return 1 - smoothProgress;
}

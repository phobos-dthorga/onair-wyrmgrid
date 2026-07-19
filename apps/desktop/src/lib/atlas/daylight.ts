export type SolarPosition = {
  latitude: number;
  longitude: number;
  equationOfTimeMinutes: number;
};

export type DaylightShadeBand =
  "civil_twilight" | "nautical_twilight" | "astronomical_twilight" | "night";

type Coordinate = [number, number];

type DaylightFeature = {
  type: "Feature";
  geometry:
    | { type: "MultiPolygon"; coordinates: Coordinate[][][] }
    | { type: "MultiLineString"; coordinates: Coordinate[][] };
  properties:
    | { kind: "shade"; band: DaylightShadeBand }
    | { kind: "terminator"; band: "geometric_horizon" };
};

export type DaylightFeatureCollection = {
  type: "FeatureCollection";
  features: DaylightFeature[];
};

const RADIANS = Math.PI / 180;
const DEGREES = 180 / Math.PI;
const MILLISECONDS_PER_DAY = 86_400_000;
const MINIMUM_SEGMENTS = 24;
const MAXIMUM_SEGMENTS = 720;

const DAYLIGHT_SHADE_BANDS: readonly DaylightShadeBand[] = [
  "night",
  "astronomical_twilight",
  "nautical_twilight",
  "civil_twilight",
];

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function normalizeLongitude(longitude: number): number {
  return ((((longitude + 180) % 360) + 360) % 360) - 180;
}

function validDate(value: Date): boolean {
  return Number.isFinite(value.getTime());
}

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

function dayOfYear(value: Date): number {
  return (
    Math.floor(
      (Date.UTC(
        value.getUTCFullYear(),
        value.getUTCMonth(),
        value.getUTCDate(),
      ) -
        Date.UTC(value.getUTCFullYear(), 0, 1)) /
        MILLISECONDS_PER_DAY,
    ) + 1
  );
}

/**
 * NOAA's compact fractional-year solar equations. The result is geometric and
 * deliberately excludes the local atmospheric-refraction correction used by
 * apparent sunrise/sunset calculators.
 */
export function solarPositionAt(value: Date): SolarPosition {
  if (!validDate(value)) throw new RangeError("A valid UTC time is required.");
  const yearDays = isLeapYear(value.getUTCFullYear()) ? 366 : 365;
  const utcHours =
    value.getUTCHours() +
    value.getUTCMinutes() / 60 +
    value.getUTCSeconds() / 3_600 +
    value.getUTCMilliseconds() / 3_600_000;
  const fractionalYear =
    ((2 * Math.PI) / yearDays) * (dayOfYear(value) - 1 + (utcHours - 12) / 24);
  const equationOfTimeMinutes =
    229.18 *
    (0.000075 +
      0.001868 * Math.cos(fractionalYear) -
      0.032077 * Math.sin(fractionalYear) -
      0.014615 * Math.cos(2 * fractionalYear) -
      0.040849 * Math.sin(2 * fractionalYear));
  const declination =
    0.006918 -
    0.399912 * Math.cos(fractionalYear) +
    0.070257 * Math.sin(fractionalYear) -
    0.006758 * Math.cos(2 * fractionalYear) +
    0.000907 * Math.sin(2 * fractionalYear) -
    0.002697 * Math.cos(3 * fractionalYear) +
    0.00148 * Math.sin(3 * fractionalYear);
  const utcMinutes = utcHours * 60;
  return {
    latitude: declination * DEGREES,
    longitude: normalizeLongitude(
      (720 - utcMinutes - equationOfTimeMinutes) / 4,
    ),
    equationOfTimeMinutes,
  };
}

export function solarAltitudeDegrees(
  value: Date,
  longitude: number,
  latitude: number,
): number {
  if (
    !Number.isFinite(longitude) ||
    !Number.isFinite(latitude) ||
    longitude < -180 ||
    longitude > 180 ||
    latitude < -90 ||
    latitude > 90
  ) {
    throw new RangeError("Valid WGS84 coordinates are required.");
  }
  return solarAltitudeFromPosition(solarPositionAt(value), longitude, latitude);
}

export function daylightBandForSolarAltitude(
  altitudeDegrees: number,
): DaylightShadeBand | "day" {
  if (!Number.isFinite(altitudeDegrees)) {
    throw new RangeError("A finite solar altitude is required.");
  }
  if (altitudeDegrees >= 0) return "day";
  if (altitudeDegrees >= -6) return "civil_twilight";
  if (altitudeDegrees >= -12) return "nautical_twilight";
  if (altitudeDegrees >= -18) return "astronomical_twilight";
  return "night";
}

function destinationPoint(
  centre: Coordinate,
  radiusDegrees: number,
  bearingDegrees: number,
): Coordinate {
  const latitude = centre[1] * RADIANS;
  const longitude = centre[0] * RADIANS;
  const radius = radiusDegrees * RADIANS;
  const bearing = bearingDegrees * RADIANS;
  const destinationLatitude = Math.asin(
    Math.sin(latitude) * Math.cos(radius) +
      Math.cos(latitude) * Math.sin(radius) * Math.cos(bearing),
  );
  const destinationLongitude =
    longitude +
    Math.atan2(
      Math.sin(bearing) * Math.sin(radius) * Math.cos(latitude),
      Math.cos(radius) - Math.sin(latitude) * Math.sin(destinationLatitude),
    );
  return [
    normalizeLongitude(destinationLongitude * DEGREES),
    clamp(destinationLatitude * DEGREES, -90, 90),
  ];
}

function solarAltitudeFromPosition(
  sun: SolarPosition,
  longitude: number,
  latitude: number,
): number {
  const latitudeRadians = latitude * RADIANS;
  const declinationRadians = sun.latitude * RADIANS;
  const hourAngle = (longitude - sun.longitude) * RADIANS;
  const sineAltitude =
    Math.sin(latitudeRadians) * Math.sin(declinationRadians) +
    Math.cos(latitudeRadians) *
      Math.cos(declinationRadians) *
      Math.cos(hourAngle);
  return Math.asin(clamp(sineAltitude, -1, 1)) * DEGREES;
}

function splitAtAntimeridian(
  start: Coordinate,
  end: Coordinate,
): Coordinate[][] {
  const longitudeDelta = end[0] - start[0];
  if (Math.abs(longitudeDelta) <= 180) return [[start, end]];

  if (start[0] > 0) {
    const unwrappedEnd = end[0] + 360;
    const fraction = (180 - start[0]) / (unwrappedEnd - start[0]);
    const latitude = start[1] + (end[1] - start[1]) * fraction;
    return [
      [start, [180, latitude]],
      [[-180, latitude], end],
    ];
  }

  const unwrappedEnd = end[0] - 360;
  const fraction = (-180 - start[0]) / (unwrappedEnd - start[0]);
  const latitude = start[1] + (end[1] - start[1]) * fraction;
  return [
    [start, [-180, latitude]],
    [[180, latitude], end],
  ];
}

export function daylightFeatureCollection(
  value: Date,
  segments = 180,
): DaylightFeatureCollection {
  if (
    !Number.isInteger(segments) ||
    segments < MINIMUM_SEGMENTS ||
    segments > MAXIMUM_SEGMENTS
  ) {
    throw new RangeError(
      `Daylight geometry requires ${MINIMUM_SEGMENTS}-${MAXIMUM_SEGMENTS} segments.`,
    );
  }
  const sun = solarPositionAt(value);
  const antiSolar: Coordinate = [
    normalizeLongitude(sun.longitude + 180),
    -sun.latitude,
  ];
  const polygonsByBand = new Map<DaylightShadeBand, Coordinate[][][]>(
    DAYLIGHT_SHADE_BANDS.map((band) => [band, []]),
  );
  const latitudeSegments = Math.max(
    MINIMUM_SEGMENTS / 2,
    Math.ceil(segments / 2),
  );
  const longitudeStep = 360 / segments;
  const latitudeStep = 180 / latitudeSegments;

  // Small WGS84 cells avoid the globe renderer's ambiguous polygon winding at
  // the poles and antimeridian. Grouping them in four MultiPolygons keeps the
  // GeoJSON source compact while ensuring no face can span the wrong hemisphere.
  for (
    let latitudeIndex = 0;
    latitudeIndex < latitudeSegments;
    latitudeIndex += 1
  ) {
    const south = -90 + latitudeIndex * latitudeStep;
    const north = south + latitudeStep;
    const centreLatitude = south + latitudeStep / 2;
    for (
      let longitudeIndex = 0;
      longitudeIndex < segments;
      longitudeIndex += 1
    ) {
      const west = -180 + longitudeIndex * longitudeStep;
      const east = west + longitudeStep;
      const centreLongitude = west + longitudeStep / 2;
      const band = daylightBandForSolarAltitude(
        solarAltitudeFromPosition(sun, centreLongitude, centreLatitude),
      );
      if (band === "day") continue;
      polygonsByBand.get(band)?.push([
        [
          [west, south],
          [east, south],
          [east, north],
          [west, north],
          [west, south],
        ],
      ]);
    }
  }

  const shadeFeatures: DaylightFeature[] = DAYLIGHT_SHADE_BANDS.map((band) => ({
    type: "Feature",
    geometry: {
      type: "MultiPolygon",
      coordinates: polygonsByBand.get(band) ?? [],
    },
    properties: { kind: "shade", band },
  }));
  const terminatorSegments: Coordinate[][] = [];
  const bearingStep = 360 / segments;
  for (let index = 0; index < segments; index += 1) {
    terminatorSegments.push(
      ...splitAtAntimeridian(
        destinationPoint(antiSolar, 90, index * bearingStep),
        destinationPoint(antiSolar, 90, (index + 1) * bearingStep),
      ),
    );
  }
  return {
    type: "FeatureCollection",
    features: [
      ...shadeFeatures,
      {
        type: "Feature",
        geometry: {
          type: "MultiLineString",
          coordinates: terminatorSegments,
        },
        properties: { kind: "terminator", band: "geometric_horizon" },
      },
    ],
  };
}

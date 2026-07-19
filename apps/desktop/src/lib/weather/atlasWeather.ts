import type { FlightCategory, FlightWeatherMapView } from "$lib/weather/types";

export type WeatherEffect =
  "none" | "rain" | "snow" | "convective" | "dust" | "obscuration";

export type WeatherStationFeatureCollection = {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    geometry: { type: "Point"; coordinates: [number, number] };
    properties: {
      id: string;
      station_icao: string;
      role: "origin" | "destination" | "alternate";
      category: FlightCategory;
      has_metar: boolean;
      has_taf: boolean;
      severity: number;
      effect: WeatherEffect;
      intensity: number;
      wind_speed_kt: number;
      wind_gust_kt: number;
      wind_bearing: number;
    };
  }>;
};

type WeatherWindProperties = {
  id: string;
  station_icao: string;
  wind_speed_kt: number;
  wind_gust_kt: number;
  bearing: number;
};

export type WeatherWindFeatureCollection = {
  type: "FeatureCollection";
  features: Array<
    | {
        type: "Feature";
        geometry: {
          type: "LineString";
          coordinates: [[number, number], [number, number]];
        };
        properties: WeatherWindProperties & { feature_type: "wind_path" };
      }
    | {
        type: "Feature";
        geometry: { type: "Point"; coordinates: [number, number] };
        properties: WeatherWindProperties & { feature_type: "wind_tip" };
      }
  >;
};

const CATEGORY_SEVERITY: Record<FlightCategory, number> = {
  vfr: 0.2,
  mvfr: 0.42,
  ifr: 0.72,
  lifr: 1,
  unknown: 0.12,
};

const TO_RADIANS = Math.PI / 180;
const TO_DEGREES = 180 / Math.PI;
const EARTH_RADIUS_NM = 3_440.065;

export function weatherEffect(
  presentWeather: string | undefined,
): WeatherEffect {
  const weather = presentWeather?.toUpperCase().replaceAll(" ", "") ?? "";
  if (!weather) return "none";
  if (
    weather.includes("TS") ||
    weather.includes("FC") ||
    weather.includes("SQ")
  ) {
    return "convective";
  }
  if (/(SN|SG|IC|PL|GR|GS)/.test(weather)) return "snow";
  if (/(RA|DZ)/.test(weather)) return "rain";
  if (/(DS|SS|DU|SA|VA)/.test(weather)) return "dust";
  if (/(FG|BR|HZ|FU)/.test(weather)) return "obscuration";
  return "none";
}

export function weatherIntensity(presentWeather: string | undefined): number {
  const weather = presentWeather?.trim() ?? "";
  if (!weather) return 0;
  if (weather.startsWith("-")) return 0.38;
  if (weather.startsWith("+")) return 1;
  return 0.68;
}

function destinationCoordinate(
  origin: [number, number],
  bearingDegrees: number,
  distanceNm: number,
): [number, number] {
  const angularDistance = distanceNm / EARTH_RADIUS_NM;
  const bearing = bearingDegrees * TO_RADIANS;
  const latitude = origin[1] * TO_RADIANS;
  const longitude = origin[0] * TO_RADIANS;
  const destinationLatitude = Math.asin(
    Math.sin(latitude) * Math.cos(angularDistance) +
      Math.cos(latitude) * Math.sin(angularDistance) * Math.cos(bearing),
  );
  const destinationLongitude =
    longitude +
    Math.atan2(
      Math.sin(bearing) * Math.sin(angularDistance) * Math.cos(latitude),
      Math.cos(angularDistance) -
        Math.sin(latitude) * Math.sin(destinationLatitude),
    );
  return [
    ((destinationLongitude * TO_DEGREES + 540) % 360) - 180,
    destinationLatitude * TO_DEGREES,
  ];
}

export function weatherStationFeatures(
  view: FlightWeatherMapView | undefined,
): WeatherStationFeatureCollection {
  return {
    type: "FeatureCollection",
    features: (view?.stations ?? []).flatMap((station) =>
      station.location
        ? [
            {
              type: "Feature" as const,
              geometry: {
                type: "Point" as const,
                coordinates: [
                  station.location.longitude,
                  station.location.latitude,
                ] as [number, number],
              },
              properties: {
                id: station.id,
                station_icao: station.station_icao,
                role: station.role,
                category: station.metar?.value.flight_category ?? "unknown",
                has_metar: station.metar !== undefined,
                has_taf: station.taf !== undefined,
                severity:
                  CATEGORY_SEVERITY[
                    station.metar?.value.flight_category ?? "unknown"
                  ],
                effect: weatherEffect(station.metar?.value.present_weather),
                intensity: weatherIntensity(
                  station.metar?.value.present_weather,
                ),
                wind_speed_kt: station.metar?.value.wind_speed_kt ?? 0,
                wind_gust_kt: station.metar?.value.wind_gust_kt ?? 0,
                wind_bearing:
                  station.metar?.value.wind_direction?.kind === "degrees"
                    ? (station.metar.value.wind_direction.value + 180) % 360
                    : 0,
              },
            },
          ]
        : [],
    ),
  };
}

export function weatherWindFeatures(
  view: FlightWeatherMapView | undefined,
): WeatherWindFeatureCollection {
  return {
    type: "FeatureCollection",
    features: (view?.stations ?? []).flatMap((station) => {
      const location = station.location;
      const metar = station.metar?.value;
      if (
        !location ||
        metar?.wind_direction?.kind !== "degrees" ||
        !metar.wind_speed_kt
      ) {
        return [];
      }

      const origin: [number, number] = [location.longitude, location.latitude];
      const bearing = (metar.wind_direction.value + 180) % 360;
      // The vector is a direction glyph, not an advection forecast. Scale it
      // for legibility while keeping its bearing tied to the sourced METAR.
      const distanceNm = 24 + Math.min(metar.wind_speed_kt, 40) * 2.6;
      const destination = destinationCoordinate(origin, bearing, distanceNm);
      const properties: WeatherWindProperties = {
        id: station.id,
        station_icao: station.station_icao,
        wind_speed_kt: metar.wind_speed_kt,
        wind_gust_kt: metar.wind_gust_kt ?? 0,
        bearing,
      };
      return [
        {
          type: "Feature" as const,
          geometry: {
            type: "LineString" as const,
            coordinates: [origin, destination] as [
              [number, number],
              [number, number],
            ],
          },
          properties: { ...properties, feature_type: "wind_path" as const },
        },
        {
          type: "Feature" as const,
          geometry: {
            type: "Point" as const,
            coordinates: destination,
          },
          properties: { ...properties, feature_type: "wind_tip" as const },
        },
      ];
    }),
  };
}

export function weatherPointCoordinates(
  view: FlightWeatherMapView | undefined,
  stationId: string | undefined,
): [number, number] | undefined {
  const station = view?.stations.find(({ id }) => id === stationId);
  return station?.location
    ? [station.location.longitude, station.location.latitude]
    : undefined;
}

export function weatherFitCoordinates(
  view: FlightWeatherMapView | undefined,
): [number, number][] {
  return weatherStationFeatures(view).features.map(
    (feature) => feature.geometry.coordinates,
  );
}

export function weatherMapSignature(
  view: FlightWeatherMapView | undefined,
): string {
  if (!view) return "";
  return [
    view.plan_id,
    view.weather_snapshot_id,
    ...view.stations.map(
      (station) =>
        `${station.id}:${station.location?.longitude ?? "?"}:${station.location?.latitude ?? "?"}:${station.metar?.value.observed_at ?? "no-metar"}:${station.metar?.value.flight_category ?? "unknown"}:${station.metar?.value.wind_direction?.kind === "degrees" ? station.metar.value.wind_direction.value : (station.metar?.value.wind_direction?.kind ?? "no-wind-direction")}:${station.metar?.value.wind_speed_kt ?? "no-wind-speed"}:${station.metar?.value.wind_gust_kt ?? "no-gust"}:${station.metar?.value.present_weather ?? "no-present-weather"}:${station.taf?.value.valid_to ?? "no-taf"}`,
    ),
  ].join("|");
}

import type { FlightCategory, FlightWeatherMapView } from "$lib/weather/types";

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
    };
  }>;
};

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
              },
            },
          ]
        : [],
    ),
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
        `${station.id}:${station.location?.longitude ?? "?"}:${station.location?.latitude ?? "?"}:${station.metar?.value.observed_at ?? "no-metar"}:${station.metar?.value.flight_category ?? "unknown"}:${station.taf?.value.valid_to ?? "no-taf"}`,
    ),
  ].join("|");
}

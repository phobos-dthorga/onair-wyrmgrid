import type { Coordinates, Observation } from "$lib/operational/types";

export type FlightCategory = "vfr" | "mvfr" | "ifr" | "lifr" | "unknown";

export type MetarObservation = {
  observed_at: string;
  raw_text: string;
  report_type?: string;
  flight_category?: FlightCategory;
  wind_direction?: { kind: "degrees"; value: number } | { kind: "variable" };
  wind_speed_kt?: number;
  wind_gust_kt?: number;
  visibility_sm?: string;
  temperature_c?: number;
  dewpoint_c?: number;
  altimeter_hpa?: number;
  present_weather?: string;
};

export type TafForecast = {
  issued_at: string;
  valid_from: string;
  valid_to: string;
  raw_text: string;
};

export type AirportWeather = {
  station_icao: string;
  metar?: Observation<MetarObservation>;
  taf?: Observation<TafForecast>;
};

export type WeatherSnapshot = {
  schema_version: number;
  id: string;
  airports: AirportWeather[];
};

export type FlightWeatherMapStation = AirportWeather & {
  id: string;
  role: "origin" | "destination" | "alternate";
  location?: Coordinates;
};

export type FlightWeatherMapView = {
  schema_version: number;
  plan_id: string;
  weather_snapshot_id: string;
  stations: FlightWeatherMapStation[];
};

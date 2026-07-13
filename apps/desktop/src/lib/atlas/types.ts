export type Coordinates = {
  latitude: number;
  longitude: number;
};

export type AirportSummary = {
  id: string;
  icao: string | null;
  name: string | null;
  location: Coordinates | null;
};

export type AircraftSummary = {
  id: string;
  registration: string | null;
  model: string | null;
  location: Coordinates | null;
  current_airport: AirportSummary | null;
};

export type FleetSnapshot = {
  value: AircraftSummary[];
  provenance: {
    kind: "on_air_fact" | "calculated";
    source: string;
    observed_at: string;
  };
};

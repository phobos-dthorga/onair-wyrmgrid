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

export type FleetSyncTrigger = "initial" | "manual" | "automatic";

export type FleetSnapshotAvailability =
  | "live"
  | "cached"
  | "offline"
  | "preview";

export type FleetSnapshotStorage = "hoard" | "memory_only" | "preview";

export type FleetSnapshotView = {
  company: {
    name: string;
    airline_code: string;
  };
  snapshot: FleetSnapshot;
  availability: FleetSnapshotAvailability;
  storage: FleetSnapshotStorage;
};

export type FleetSyncResult = {
  disposition: "synchronized" | "quietly_ignored";
  snapshot: FleetSnapshotView | null;
};

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

export type FboSummary = {
  id: string;
  name: string | null;
  airport: AirportSummary | null;
};

export type FleetSnapshot = {
  value: AircraftSummary[];
  provenance: {
    kind: "on_air_fact" | "external_fact" | "calculated" | "recommendation";
    source: string;
    observed_at: string;
  };
};

export type DataSyncTrigger = "initial" | "manual" | "automatic";

export type SnapshotAvailability = "live" | "cached" | "offline" | "preview";

export type SnapshotStorage = "hoard" | "memory_only" | "preview";

export type FleetSnapshotView = {
  company: {
    name: string;
    airline_code: string;
  };
  snapshot: FleetSnapshot;
  availability: SnapshotAvailability;
  storage: SnapshotStorage;
};

export type FboSnapshot = {
  value: FboSummary[];
  provenance: FleetSnapshot["provenance"];
};

export type FboSnapshotView = {
  company: FleetSnapshotView["company"];
  snapshot: FboSnapshot;
  availability: SnapshotAvailability;
  storage: SnapshotStorage;
};

export type CompanyDataSyncResult = {
  disposition: "synchronized" | "quietly_ignored";
  fleet: FleetSnapshotView | null;
  fbos: FboSnapshotView | null;
  failures: Array<{
    resource: "fleet" | "fbos";
    message: string;
  }>;
};

export type Coordinates = {
  latitude: number;
  longitude: number;
};

export type AtlasRoutePoint = {
  location: Coordinates;
  label?: string;
  source_sequence?: number;
  observed_at?: string;
  gap_before: boolean;
};

export type AtlasPlannedPoint = {
  id: string;
  kind: "origin" | "route_leg" | "destination" | "alternate";
  label: string;
  sequence?: number;
  airway?: string;
  location?: Coordinates;
  on_route: boolean;
  gap_before: boolean;
};

export type AtlasPlannedRoute = {
  schema_version: number;
  plan_id: string;
  origin_icao: string;
  destination_icao: string;
  airac?: string;
  source_text?: string;
  provenance: {
    kind:
      | "on_air_fact"
      | "external_fact"
      | "external_calculation"
      | "calculated"
      | "recommendation";
    provider: string;
    provider_revision?: string;
    generated_at?: string;
    retrieved_at: string;
    transformation_version: number;
    freshness: "current" | "stale" | "unknown";
  };
  points: AtlasPlannedPoint[];
};

export type AtlasRecordedRoute = {
  source_sample_count: number;
  represented_point_count: number;
  method: "exact" | "min_max_envelope";
  points: AtlasRoutePoint[];
};

export type AtlasFlightRoute = {
  schema_version: number;
  session_id: string;
  context?: "recording" | "dispatch_plan";
  planned?: AtlasPlannedRoute;
  recorded?: AtlasRecordedRoute;
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
    kind:
      | "on_air_fact"
      | "external_fact"
      | "external_calculation"
      | "calculated"
      | "recommendation";
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

export type JobLeg = {
  id: string;
  sequence: number;
  kind: "cargo" | "passengers";
  departure: AirportSummary | null;
  destination: AirportSummary | null;
  current_airport: AirportSummary | null;
  cargo_weight_lb?: number;
  passengers?: number;
  distance_nm?: number;
  description?: string;
};

export type JobSummary = {
  id: string;
  mission_type?: string;
  description?: string;
  reported_pay?: number;
  created_at?: string;
  taken_at?: string;
  expires_at?: string;
  legs: JobLeg[];
};

export type JobSnapshotView = {
  company: FleetSnapshotView["company"];
  snapshot: {
    value: { schema_version: number; jobs: JobSummary[] };
    provenance: FleetSnapshot["provenance"];
  };
  availability: SnapshotAvailability;
  storage: SnapshotStorage;
};

export type CompanyDataSyncResult = {
  disposition: "synchronized" | "quietly_ignored";
  fleet: FleetSnapshotView | null;
  fbos: FboSnapshotView | null;
  jobs: JobSnapshotView | null;
  failures: Array<{
    resource: "fleet" | "fbos" | "jobs";
    message: string;
  }>;
};

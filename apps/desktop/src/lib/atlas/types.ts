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

export type AircraftClassQualification = {
  id: string;
  aircraft_class_id: string;
  short_name?: string;
  name?: string;
  last_validated_at?: string;
};

export type StaffMemberSummary = {
  id: string;
  display_name?: string;
  avatar_reference?: string;
  category_code?: number;
  status_code?: number;
  current_airport?: AirportSummary;
  home_airport?: AirportSummary;
  busy_until?: string;
  is_online?: boolean;
  class_qualifications: AircraftClassQualification[];
};

export type StaffSnapshotView = {
  company: FleetSnapshotView["company"];
  snapshot: {
    value: { schema_version: number; staff: StaffMemberSummary[] };
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
  staff: StaffSnapshotView | null;
  failures: Array<{
    resource: "fleet" | "fbos" | "jobs" | "staff";
    message: string;
  }>;
};

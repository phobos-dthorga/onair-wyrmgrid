export type ProvenanceKind =
  | "on_air_fact"
  | "external_fact"
  | "external_calculation"
  | "calculated"
  | "recommendation";

export type Coordinates = { latitude: number; longitude: number };
export type MassUnit = "kilograms" | "pounds";
export type Mass = { value: number; unit: MassUnit };

export type OperationalProvenance = {
  kind: ProvenanceKind;
  provider: string;
  provider_revision?: string;
  generated_at?: string;
  retrieved_at: string;
  transformation_version: number;
  freshness: "current" | "stale" | "unknown";
};

export type Observation<T> = {
  value: T;
  provenance: OperationalProvenance;
};

export type FlightPlanAirport = {
  icao: string;
  name?: string;
  location?: Coordinates;
  planned_runway?: string;
};

export type FlightPlanSnapshot = {
  schema_version: number;
  id: string;
  identity: Observation<{
    airac?: string;
    provider_plan_reference?: string;
  }>;
  airports: Observation<{
    origin: FlightPlanAirport;
    destination: FlightPlanAirport;
    alternates: FlightPlanAirport[];
  }>;
  aircraft?: Observation<{
    icao_type?: string;
    registration?: string;
    model?: string;
  }>;
  schedule?: Observation<{
    scheduled_out?: string;
    scheduled_off?: string;
    scheduled_on?: string;
    scheduled_in?: string;
    estimated_enroute_seconds?: number;
  }>;
  weights?: Observation<{
    payload?: Mass;
    zero_fuel?: Mass;
    takeoff?: Mass;
    landing?: Mass;
  }>;
  fuel?: Observation<{
    taxi?: Mass;
    enroute?: Mass;
    reserve?: Mass;
    alternate?: Mass;
    contingency?: Mass;
    extra?: Mass;
    ramp?: Mass;
    takeoff?: Mass;
    landing?: Mass;
  }>;
  route?: Observation<{
    source_text?: string;
    initial_altitude_ft?: number;
    distance_nm?: number;
    legs: Array<{
      sequence: number;
      ident: string;
      airway?: string;
      location?: Coordinates;
    }>;
  }>;
};

export type DispatchStatus = {
  provider_available: boolean;
  availability: "empty" | "ready";
  persistence: "session_only";
  importing: boolean;
  snapshot?: FlightPlanSnapshot;
};

export type SimBriefReferenceKind = "pilot_id" | "username";

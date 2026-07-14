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

export type DispatchFindingStatus =
  "match" | "difference" | "information" | "unavailable";

export type DispatchFinding = {
  category:
    | "aircraft_identity"
    | "aircraft_model"
    | "aircraft_position"
    | "payload"
    | "schedule";
  status: DispatchFindingStatus;
  title: string;
  explanation: string;
  plan_value?: string;
  onair_value?: string;
};

export type DispatchComparison = {
  fleet_available: boolean;
  fleet_observed_at?: string;
  matched_aircraft?: {
    basis: "registration" | "exact_model";
    registration?: string;
    model?: string;
    current_airport_icao?: string;
  };
  findings: DispatchFinding[];
  provenance: OperationalProvenance;
};

export type WeatherSnapshot = {
  schema_version: number;
  id: string;
  airports: Array<{
    station_icao: string;
    metar?: Observation<{
      observed_at: string;
      raw_text: string;
      report_type?: string;
      flight_category?: "vfr" | "mvfr" | "ifr" | "lifr" | "unknown";
      wind_direction?:
        { kind: "degrees"; value: number } | { kind: "variable" };
      wind_speed_kt?: number;
      wind_gust_kt?: number;
      visibility_sm?: string;
      temperature_c?: number;
      dewpoint_c?: number;
      altimeter_hpa?: number;
      present_weather?: string;
    }>;
    taf?: Observation<{
      issued_at: string;
      valid_from: string;
      valid_to: string;
      raw_text: string;
    }>;
  }>;
};

export type DispatchStatus = {
  provider_available: boolean;
  availability: "empty" | "ready";
  persistence: "session_only";
  importing: boolean;
  snapshot?: FlightPlanSnapshot;
  comparison?: DispatchComparison;
  weather: {
    provider_available: boolean;
    availability: "not_requested" | "ready";
    refreshing: boolean;
    cache: "none" | "fresh" | "expired";
    snapshot?: WeatherSnapshot;
  };
};

export type SimBriefReferenceKind = "pilot_id" | "username";

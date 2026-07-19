import type {
  FlightOperationContextChange,
  FlightOperationJourneyView,
  FlightOperationView,
} from "$lib/flightOperation/types";
import type {
  Coordinates,
  Observation,
  OperationalProvenance,
  ProvenanceKind,
} from "$lib/operational/types";
import type { FlightWeatherMapView, WeatherSnapshot } from "$lib/weather/types";
import type { GlobalWeatherCondition } from "$lib/forge/types";

export type {
  Coordinates,
  Observation,
  OperationalProvenance,
  ProvenanceKind,
} from "$lib/operational/types";
export type { WeatherSnapshot } from "$lib/weather/types";

export type MassUnit = "kilograms" | "pounds";
export type Mass = { value: number; unit: MassUnit };

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
    | "schedule"
    | "job_route";
  status: DispatchFindingStatus;
  message_key: string;
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

export type RouteWeatherAvailability =
  "ready" | "partial" | "route_unavailable" | "source_unavailable";

export type RouteWeatherSourceSample = {
  point_id: string;
  location: Coordinates;
  support_distance_nm: number;
  condition: GlobalWeatherCondition;
  temperature_c?: number;
  precipitation_mm?: number;
  cloud_cover_percent?: number;
  wind_direction_degrees?: number;
  wind_speed_kt?: number;
};

export type RouteWeatherSample = {
  id: string;
  segment_index: number;
  distance_from_origin_nm: number;
  location: Coordinates;
  source?: RouteWeatherSourceSample;
};

export type RouteWeatherLayerAnalysis = {
  layer_id: string;
  title: string;
  provenance: OperationalProvenance;
  availability: RouteWeatherAvailability;
  samples: RouteWeatherSample[];
};

export type RouteWeatherAnalysis = {
  schema_version: number;
  plan_id: string;
  sample_interval_nm: number;
  maximum_support_distance_nm: number;
  mapped_route_point_count: number;
  unresolved_route_point_count: number;
  availability: RouteWeatherAvailability;
  layers: RouteWeatherLayerAnalysis[];
};

export type DispatchStatus = {
  provider_available: boolean;
  availability: "empty" | "ready";
  persistence: "session_only";
  importing: boolean;
  snapshot?: FlightPlanSnapshot;
  atlas_plan?: import("$lib/atlas/types").AtlasPlannedRoute;
  atlas_weather?: FlightWeatherMapView;
  route_weather?: RouteWeatherAnalysis;
  journey: FlightOperationJourneyView;
  atlas_route?: AtlasRouteView;
  comparison?: DispatchComparison;
  selected_job?: {
    job: import("$lib/atlas/types").JobSummary;
    observed_at: string;
    availability: "live" | "cached" | "offline";
  };
  operation?: FlightOperationView;
  operation_change: FlightOperationContextChange;
  weather: {
    provider_available: boolean;
    availability: "not_requested" | "ready";
    refreshing: boolean;
    cache: "none" | "fresh" | "expired";
    snapshot?: WeatherSnapshot;
  };
};

export type AtlasRouteFeatureKind =
  "origin" | "route_fix" | "destination" | "alternate";

export type AtlasRouteFeature = {
  id: string;
  kind: AtlasRouteFeatureKind;
  ident: string;
  name?: string;
  sequence?: number;
  airway?: string;
  availability: "resolved" | "location_unavailable";
  location?: Coordinates;
};

export type AtlasRouteView = {
  projection_version: number;
  plan_id: string;
  airac?: string;
  source_text?: string;
  route_feature_ids: string[];
  features: AtlasRouteFeature[];
  mapped_route_feature_count: number;
  unresolved_route_feature_count: number;
  provenance: OperationalProvenance;
};

export type SimBriefReferenceKind = "pilot_id" | "username";

export type SimBriefAccountPreference = {
  reference_kind: SimBriefReferenceKind;
  reference: string;
};

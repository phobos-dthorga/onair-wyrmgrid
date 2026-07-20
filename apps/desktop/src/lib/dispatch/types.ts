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
import type { GlobalWeatherTimeScope } from "$lib/forge/types";

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
    | "manifest_coverage"
    | "aircraft_seats"
    | "aircraft_payload_capacity"
    | "aircraft_configuration"
    | "aircraft_availability"
    | "aircraft_assignment"
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
    id: string;
    basis: "registration" | "exact_model" | "reviewed_assignment";
    registration?: string;
    model?: string;
    current_airport_icao?: string;
  };
  findings: DispatchFinding[];
  provenance: OperationalProvenance;
};

export type RouteWeatherAvailability =
  "ready" | "partial" | "route_unavailable" | "source_unavailable";

export type RouteWeatherTimingAvailability =
  "ready" | "departure_unavailable" | "duration_unavailable";

export type RouteWeatherTiming = {
  availability: RouteWeatherTimingAvailability;
  departure_basis?: "scheduled_off" | "scheduled_out";
  duration_basis?: "estimated_enroute" | "scheduled_on" | "scheduled_in";
  departure_at?: string;
  duration_seconds?: number;
};

export type RouteWeatherTemporalSupport = "eta_matched" | "current_context";
export type RouteWeatherTemporalMode = "live" | "historical";

export type RouteWeatherSourceSample = {
  point_id: string;
  location: Coordinates;
  support_distance_nm: number;
  temporal_support: RouteWeatherTemporalSupport;
  valid_at?: string;
  time_offset_seconds?: number;
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
  estimated_arrival_at?: string;
  source?: RouteWeatherSourceSample;
};

export type RouteWeatherRadarContext = {
  layer_id: string;
  title: string;
  provenance: OperationalProvenance;
  frame_time: string;
  relationship: "observation_only";
};

export type RouteWeatherLayerAnalysis = {
  layer_id: string;
  title: string;
  provenance: OperationalProvenance;
  time_scope?: GlobalWeatherTimeScope;
  availability: RouteWeatherAvailability;
  samples: RouteWeatherSample[];
};

export type RouteWeatherAnalysis = {
  schema_version: number;
  plan_id: string;
  sample_interval_nm: number;
  maximum_support_distance_nm: number;
  maximum_temporal_support_seconds: number;
  mapped_route_point_count: number;
  unresolved_route_point_count: number;
  timing: RouteWeatherTiming;
  temporal_mode: RouteWeatherTemporalMode;
  availability: RouteWeatherAvailability;
  layers: RouteWeatherLayerAnalysis[];
  radar_contexts: RouteWeatherRadarContext[];
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
    time_basis: RouteWeatherTemporalMode;
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

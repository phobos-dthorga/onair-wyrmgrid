export type BridgeCapability =
  | "telemetry_read"
  | "active_plan_read"
  | "flight_plan_load"
  | "command_execute";

export type SimulatorProviderProcessState =
  "unavailable" | "stopped" | "starting" | "running" | "stopping" | "failed";

export type ProviderConnectionState =
  | "starting"
  | "waiting_for_simulator"
  | "connected"
  | "disconnected"
  | "stopped"
  | "failed"
  | "unavailable";

export type SimulatorProviderView = {
  id: string;
  name: string;
  version: string;
  simulators: string[];
  capabilities: BridgeCapability[];
  process_state: SimulatorProviderProcessState;
  connection_state: ProviderConnectionState;
  last_code?: string;
  telemetry_stale: boolean;
  latest_snapshot_age_seconds?: number;
  connected_age_seconds?: number;
};

export type SimulatorPreferences = {
  selected_provider_id?: string;
  start_with_wyrmgrid: boolean;
};

export const defaultSimulatorPreferences: SimulatorPreferences = {
  start_with_wyrmgrid: false,
};

export type SimulatorTelemetrySnapshot = {
  schema_version: number;
  sequence: number;
  provenance: {
    kind: "external_fact";
    provider: string;
    provider_revision?: string;
    generated_at?: string;
    retrieved_at: string;
    transformation_version: number;
    freshness: "current" | "stale" | "unknown";
  };
  simulator: {
    family: string;
    version?: string;
  };
  aircraft: {
    title: string;
    registration?: string;
  };
  position: {
    latitude: number;
    longitude: number;
  };
  altitude_feet: number;
  pitch_degrees: number;
  bank_degrees: number;
  true_heading_degrees: number;
  indicated_airspeed_knots: number;
  true_airspeed_knots: number;
  ground_speed_knots: number;
  on_ground: boolean;
  simulation_time_utc?: string;
  fuel_total_gallons?: number;
  fuel_total_weight_pounds?: number;
  gross_weight_pounds?: number;
  engines_running?: boolean;
  parking_brake_set?: boolean;
  paused?: boolean;
  simulation_rate?: number;
};

export type SimulatorBridgeView = {
  bridge_protocol_version: number;
  telemetry_schema_version: number;
  providers: SimulatorProviderView[];
  active_provider_id?: string;
  latest_snapshot?: SimulatorTelemetrySnapshot;
};

export const emptySimulatorBridge: SimulatorBridgeView = {
  bridge_protocol_version: 1,
  telemetry_schema_version: 1,
  providers: [],
};

export type SimulatorRecordingPreferences = {
  retention_days: number;
};

export const defaultSimulatorRecordingPreferences: SimulatorRecordingPreferences =
  {
    retention_days: 30,
  };

export type SimulatorRecordingStatus = "active" | "completed" | "interrupted";

export type SimulatorSessionSummary = {
  id: string;
  provider_id: string;
  simulator_family: string;
  simulator_version?: string;
  aircraft_title: string;
  aircraft_registration?: string;
  started_at: string;
  ended_at?: string;
  status: SimulatorRecordingStatus;
  sample_count: number;
};

export type SimulatorRecordedSample = {
  source_sequence: number;
  observed_at: string;
  simulation_time_utc?: string;
  altitude_feet: number;
  indicated_airspeed_knots: number;
  true_airspeed_knots: number;
  ground_speed_knots: number;
  fuel_total_weight_pounds?: number;
  gross_weight_pounds?: number;
  pitch_degrees: number;
  bank_degrees: number;
  gap_before: boolean;
};

export type SimulatorRecordingView = {
  preferences: SimulatorRecordingPreferences;
  active_session_id?: string;
  sessions: SimulatorSessionSummary[];
  last_code?: string;
};

export type SimulatorSessionView = {
  session: SimulatorSessionSummary;
  samples: SimulatorRecordedSample[];
  sample_window_limit: number;
};

export const emptySimulatorRecording: SimulatorRecordingView = {
  preferences: defaultSimulatorRecordingPreferences,
  sessions: [],
};

import type { AtlasFlightRoute } from "$lib/atlas/types";

export type BridgeCapability =
  | "telemetry_read"
  | "active_plan_read"
  | "flight_plan_load"
  | "command_execute";

export type ProviderPlatform =
  "windows_x86_64" | "linux_x86_64" | "macos_aarch64" | "macos_x86_64";

export type AudioProviderCapability =
  | "source_enumeration"
  | "permission_requests"
  | "pcm_s16le_capture"
  | "level_metering"
  | "hot_plug_notifications"
  | "clock_synchronization";

export type AudioProviderPackageInspection = {
  package_schema_version: number;
  package_kind: "audio_provider";
  id: string;
  name: string;
  version: string;
  author: string;
  audio_protocol_version: number;
  platforms: ProviderPlatform[];
  capabilities: AudioProviderCapability[];
  archive_sha256: string;
  archive_size: number;
  expanded_size: number;
  file_count: number;
  publisher_verified: false;
};

export type ManagedAudioProviderPackage = {
  id: string;
  name: string;
  author: string;
  active_version: string;
  rollback_version?: string;
  enabled: boolean;
  installed_versions: string[];
  active_archive_sha256: string;
  source: "local_file" | "first_party";
  publisher_verified: false;
  audio_protocol_version: number;
  platforms: ProviderPlatform[];
  capabilities: AudioProviderCapability[];
};

export type AudioCodecCapability = "encode_pcm_s16le";

export type AudioCodecProfile = {
  id: AudioProfileId;
  codec_id: string;
  media_type: string;
  channels: number;
  sample_rate_hz: number;
  target_bitrate_bps: number;
  packet_duration_48khz_frames: number;
};

export type AudioCodecPackageInspection = {
  package_schema_version: number;
  package_kind: "audio_codec_provider";
  id: string;
  name: string;
  version: string;
  author: string;
  codec_protocol_version: number;
  platforms: ProviderPlatform[];
  capabilities: AudioCodecCapability[];
  profiles: AudioCodecProfile[];
  archive_sha256: string;
  archive_size: number;
  expanded_size: number;
  file_count: number;
  publisher_verified: false;
};

export type ManagedAudioCodecPackage = {
  id: string;
  name: string;
  author: string;
  active_version: string;
  rollback_version?: string;
  enabled: boolean;
  installed_versions: string[];
  active_archive_sha256: string;
  source: "local_file" | "first_party";
  publisher_verified: false;
  codec_protocol_version: number;
  platforms: ProviderPlatform[];
  capabilities: AudioCodecCapability[];
  profiles: AudioCodecProfile[];
};

export type SimulatorProviderPackageInspection = {
  package_schema_version: number;
  package_kind: "simulator_provider";
  id: string;
  name: string;
  version: string;
  author: string;
  bridge_protocol_version: number;
  platforms: ProviderPlatform[];
  simulators: string[];
  capabilities: BridgeCapability[];
  archive_sha256: string;
  archive_size: number;
  expanded_size: number;
  file_count: number;
  publisher_verified: false;
};

export type ManagedSimulatorProviderPackage = {
  id: string;
  name: string;
  author: string;
  active_version: string;
  rollback_version?: string;
  enabled: boolean;
  installed_versions: string[];
  active_archive_sha256: string;
  source: "local_file" | "first_party";
  publisher_verified: false;
  bridge_protocol_version: number;
  platforms: ProviderPlatform[];
  simulators: string[];
  capabilities: BridgeCapability[];
};

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
  automatic_start: boolean;
  automatic_stop: boolean;
  landing_settle_seconds: number;
};

export const defaultSimulatorRecordingPreferences: SimulatorRecordingPreferences =
  {
    retention_days: 30,
    automatic_start: false,
    automatic_stop: true,
    landing_settle_seconds: 30,
  };

export type SimulatorRecordingStatus = "active" | "completed" | "interrupted";
export type SimulatorCaptureMode = "manual" | "automatic";

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
  capture_mode: SimulatorCaptureMode;
  pinned: boolean;
  plan_associated: boolean;
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
  position?: { latitude: number; longitude: number };
  on_ground?: boolean;
  engines_running?: boolean;
  parking_brake_set?: boolean;
  paused?: boolean;
};

export type SimulatorSessionEvent = {
  id: number;
  event_kind: string;
  observed_at: string;
  source_sequence?: number;
  evidence: Record<string, unknown>;
};

export type PlannedActualComparison = {
  association: {
    correlation_version: number;
    plan_id: string;
    origin_icao: string;
    destination_icao: string;
    provider_plan_reference?: string;
  };
  analysis_complete: boolean;
  planned_enroute_seconds?: number;
  recorded_seconds?: number;
  planned_distance_nm?: number;
  recorded_track_distance_nm?: number;
  planned_initial_altitude_ft?: number;
  recorded_peak_altitude_ft?: number;
  planned_takeoff_fuel_pounds?: number;
  planned_landing_fuel_pounds?: number;
  recorded_start_fuel_pounds?: number;
  recorded_end_fuel_pounds?: number;
  recorded_fuel_used_pounds?: number;
  duration_delta_seconds?: number;
  distance_delta_nm?: number;
  altitude_delta_ft?: number;
  takeoff_fuel_delta_pounds?: number;
  landing_fuel_delta_pounds?: number;
  origin_proximity_nm?: number;
  destination_proximity_nm?: number;
  registration_matches?: boolean;
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
  sample_window_offset: number;
  has_older_samples: boolean;
  has_newer_samples: boolean;
  events: SimulatorSessionEvent[];
  comparison?: PlannedActualComparison;
};

export type SimulatorDownsamplingMethod = "exact" | "min_max_envelope";

export type SimulatorDebriefTrace = {
  source_sample_count: number;
  represented_sample_count: number;
  gap_count: number;
  method: SimulatorDownsamplingMethod;
  samples: SimulatorRecordedSample[];
};

export type SimulatorSessionDebrief = {
  schema_version: number;
  session: SimulatorSessionSummary;
  source_sample_count: number;
  traces: {
    altitude: SimulatorDebriefTrace;
    speed: SimulatorDebriefTrace;
    fuel?: SimulatorDebriefTrace;
    attitude: SimulatorDebriefTrace;
  };
  route: AtlasFlightRoute;
  comparison?: PlannedActualComparison;
};

export type SimulatorRecordingExport = {
  filename: string;
  media_type: string;
  content: string;
};

export const emptySimulatorRecording: SimulatorRecordingView = {
  preferences: defaultSimulatorRecordingPreferences,
  sessions: [],
};

export type AudioProfileId =
  "pilot_microphone_v1" | "isolated_voice_v1" | "mixed_stereo_v1";

export type AudioRecordingPreferences = {
  enabled: boolean;
  capture_manual: boolean;
  capture_automatic: boolean;
  retention_days: number;
  storage_budget_bytes: number;
};

export const defaultAudioRecordingPreferences: AudioRecordingPreferences = {
  enabled: false,
  capture_manual: false,
  capture_automatic: false,
  retention_days: 30,
  storage_budget_bytes: 5 * 1024 * 1024 * 1024,
};

export type AudioSourceSelection = {
  provider_id: string;
  source_id: string;
  profile_id: AudioProfileId;
  codec_provider_id: string;
  enabled: boolean;
  playback_muted: boolean;
  playback_solo: boolean;
  playback_volume_percent: number;
};

export type AudioSourceView = {
  id: string;
  display_name: string;
  role: string;
  availability: "available" | "unavailable";
  permission: "not_required" | "prompt_required" | "granted" | "denied";
  supported_profiles: AudioProfileId[];
  codec_provider_id?: string;
  enabled: boolean;
  playback_muted: boolean;
  playback_solo: boolean;
  playback_volume_percent: number;
  peak_millidbfs?: number;
  clipped: boolean;
};

export type AudioCodecView = {
  id: string;
  name: string;
  supported_profiles: AudioProfileId[];
};

export type AudioSessionSummary = {
  id: string;
  simulator_session_id?: string;
  provider_id: string;
  capture_mode: "manual" | "automatic";
  started_at: string;
  ended_at?: string;
  status: "active" | "completed" | "interrupted";
  media_availability: "available" | "not_in_backup" | "missing" | "tombstoned";
  total_media_bytes: number;
};

export type AudioRecordingView = {
  preferences: AudioRecordingPreferences;
  provider_id?: string;
  provider_available: boolean;
  recording_active: boolean;
  active_session_id?: string;
  sources: AudioSourceView[];
  codecs: AudioCodecView[];
  sessions: AudioSessionSummary[];
  last_code?: string;
};

export const emptyAudioRecording: AudioRecordingView = {
  preferences: defaultAudioRecordingPreferences,
  provider_available: false,
  recording_active: false,
  sources: [],
  codecs: [],
  sessions: [],
};

export type EncodedAudioPacketView = {
  sequence: string;
  provider_monotonic_ns: string;
  duration_48khz_frames: number;
  bytes: number[];
};

export type AudioTrackPlaybackView = {
  track_id: string;
  source_id: string;
  profile_id: AudioProfileId;
  codec_provider_id: string;
  codec_provider_version: string;
  codec_id: string;
  codec_media_type: string;
  playback_muted: boolean;
  playback_solo: boolean;
  playback_volume_percent: number;
  frame_count: number;
  packets: EncodedAudioPacketView[];
};

export type AudioPlaybackView = {
  session_id: string;
  authenticated: boolean;
  tracks: AudioTrackPlaybackView[];
};

export type AudioExportView = {
  filename: string;
  media_type: string;
  plaintext_warning_required: boolean;
  packet_count: number;
};

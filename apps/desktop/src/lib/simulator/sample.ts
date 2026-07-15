import type { SimulatorRecordingView, SimulatorSessionView } from "./types";

const previewSession = {
  id: "preview-flight-2026-07-14",
  provider_id: "preview.msfs2024",
  simulator_family: "MSFS 2024",
  simulator_version: "preview",
  aircraft_title: "Cessna C680 Citation Sovereign",
  aircraft_registration: "VH-GFA",
  started_at: "2026-07-14T09:20:00Z",
  ended_at: "2026-07-14T10:05:00Z",
  status: "completed" as const,
  sample_count: 8,
  capture_mode: "automatic" as const,
  pinned: false,
  plan_associated: true,
};

export const simulatorRecordingPreview: SimulatorRecordingView = {
  preferences: {
    retention_days: 30,
    automatic_start: false,
    automatic_stop: true,
    landing_settle_seconds: 30,
  },
  sessions: [previewSession],
};

export const simulatorRecordingSessionPreview: SimulatorSessionView = {
  session: previewSession,
  sample_window_limit: 600,
  sample_window_offset: 0,
  has_older_samples: false,
  has_newer_samples: false,
  events: [
    {
      id: 1,
      event_kind: "takeoff_confirmed",
      observed_at: "2026-07-14T09:20:00Z",
      source_sequence: 1,
      evidence: { on_ground: false, confirmation_samples: 2 },
    },
    {
      id: 2,
      event_kind: "telemetry_gap",
      observed_at: "2026-07-14T09:44:00Z",
      source_sequence: 8,
      evidence: { previous_sequence: 4 },
    },
    {
      id: 3,
      event_kind: "landing_settled",
      observed_at: "2026-07-14T10:05:00Z",
      source_sequence: 11,
      evidence: { on_ground: true, settle_seconds: 30 },
    },
  ],
  comparison: {
    association: {
      correlation_version: 1,
      plan_id: "preview-simbrief-plan",
      origin_icao: "YSSY",
      destination_icao: "NZAA",
    },
    analysis_complete: true,
    planned_enroute_seconds: 10_800,
    recorded_seconds: 10_950,
    planned_distance_nm: 1_184.6,
    recorded_track_distance_nm: 1_201.2,
    planned_initial_altitude_ft: 31_000,
    recorded_peak_altitude_ft: 31_000,
    planned_takeoff_fuel_pounds: 7_800,
    recorded_fuel_used_pounds: 4_160,
    origin_proximity_nm: 0.8,
    destination_proximity_nm: 1.4,
    registration_matches: true,
  },
  samples: [
    [1, "2026-07-14T09:20:00Z", 1200, 95, 101, 98, false],
    [2, "2026-07-14T09:26:00Z", 4200, 145, 153, 150, false],
    [3, "2026-07-14T09:32:00Z", 9100, 205, 218, 212, false],
    [4, "2026-07-14T09:38:00Z", 18000, 260, 285, 279, false],
    [8, "2026-07-14T09:44:00Z", 26000, 282, 320, 315, true],
    [9, "2026-07-14T09:50:00Z", 31000, 274, 336, 331, false],
    [10, "2026-07-14T09:56:00Z", 16000, 238, 270, 264, false],
    [11, "2026-07-14T10:05:00Z", 1300, 112, 118, 115, false],
  ].map(
    ([sequence, observedAt, altitude, indicated, trueSpeed, ground, gap]) => ({
      source_sequence: sequence as number,
      observed_at: observedAt as string,
      altitude_feet: altitude as number,
      indicated_airspeed_knots: indicated as number,
      true_airspeed_knots: trueSpeed as number,
      ground_speed_knots: ground as number,
      pitch_degrees: 0,
      bank_degrees: 0,
      gap_before: gap as boolean,
    }),
  ),
};

import type {
  SimulatorRecordingView,
  SimulatorSessionDebrief,
  SimulatorSessionView,
} from "./types";

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
      correlation_version: 2,
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
    planned_landing_fuel_pounds: 3_400,
    recorded_start_fuel_pounds: 7_740,
    recorded_end_fuel_pounds: 3_580,
    recorded_fuel_used_pounds: 4_160,
    duration_delta_seconds: 150,
    distance_delta_nm: 16.6,
    altitude_delta_ft: 0,
    takeoff_fuel_delta_pounds: -60,
    landing_fuel_delta_pounds: 180,
    origin_proximity_nm: 0.8,
    destination_proximity_nm: 1.4,
    registration_matches: true,
  },
  samples: [
    [1, "2026-07-14T09:20:00Z", 1200, 95, 101, 98, 7740, 4, 1, false],
    [2, "2026-07-14T09:26:00Z", 4200, 145, 153, 150, 7370, 8, 4, false],
    [3, "2026-07-14T09:32:00Z", 9100, 205, 218, 212, 6930, 5, -9, false],
    [4, "2026-07-14T09:38:00Z", 18000, 260, 285, 279, 6410, 3, 14, false],
    [8, "2026-07-14T09:44:00Z", 26000, 282, 320, 315, 5820, 2, -3, true],
    [9, "2026-07-14T09:50:00Z", 31000, 274, 336, 331, 5240, 0, 2, false],
    [10, "2026-07-14T09:56:00Z", 16000, 238, 270, 264, 4380, -4, -12, false],
    [11, "2026-07-14T10:05:00Z", 1300, 112, 118, 115, 3580, -2, 3, false],
  ].map(
    (
      [
        sequence,
        observedAt,
        altitude,
        indicated,
        trueSpeed,
        ground,
        fuel,
        pitch,
        bank,
        gap,
      ],
      index,
    ) => ({
      source_sequence: sequence as number,
      observed_at: observedAt as string,
      altitude_feet: altitude as number,
      indicated_airspeed_knots: indicated as number,
      true_airspeed_knots: trueSpeed as number,
      ground_speed_knots: ground as number,
      fuel_total_weight_pounds: fuel as number,
      pitch_degrees: pitch as number,
      bank_degrees: bank as number,
      position: {
        latitude: -33.9461 + index * 0.66,
        longitude: 151.1772 + index * 3.54,
      },
      gap_before: gap as boolean,
    }),
  ),
};

const previewTrace = {
  source_sample_count: simulatorRecordingSessionPreview.samples.length,
  represented_sample_count: simulatorRecordingSessionPreview.samples.length,
  gap_count: 1,
  method: "exact" as const,
  samples: simulatorRecordingSessionPreview.samples,
};

export const simulatorRecordingDebriefPreview: SimulatorSessionDebrief = {
  schema_version: 1,
  session: previewSession,
  source_sample_count: simulatorRecordingSessionPreview.samples.length,
  traces: {
    altitude: previewTrace,
    speed: previewTrace,
    fuel: previewTrace,
    attitude: previewTrace,
  },
  comparison: simulatorRecordingSessionPreview.comparison,
  route: {
    schema_version: 2,
    session_id: previewSession.id,
    planned: {
      schema_version: 1,
      plan_id: "preview-simbrief-plan",
      origin_icao: "YSSY",
      destination_icao: "NZAA",
      airac: "2607",
      provenance: {
        kind: "external_calculation",
        provider: "simbrief",
        provider_revision: "2607",
        retrieved_at: "2026-07-15T22:00:00Z",
        transformation_version: 1,
        freshness: "current",
      },
      points: [
        {
          id: "origin:yssy",
          kind: "origin",
          location: { latitude: -33.9461, longitude: 151.1772 },
          label: "YSSY",
          on_route: true,
          gap_before: false,
        },
        {
          id: "route:0000:mudgi",
          kind: "route_leg",
          location: { latitude: -32.9167, longitude: 151.8167 },
          label: "MUDGI",
          sequence: 0,
          on_route: true,
          gap_before: false,
        },
        {
          id: "route:0001:unresolved",
          kind: "route_leg",
          label: "UNRESOLVED",
          sequence: 1,
          on_route: true,
          gap_before: false,
        },
        {
          id: "destination:nzaa",
          kind: "destination",
          location: { latitude: -37.0081, longitude: 174.7917 },
          label: "NZAA",
          on_route: true,
          gap_before: true,
        },
      ],
    },
    recorded: {
      source_sample_count: simulatorRecordingSessionPreview.samples.length,
      represented_point_count: simulatorRecordingSessionPreview.samples.length,
      method: "exact",
      points: simulatorRecordingSessionPreview.samples.flatMap((sample) =>
        sample.position
          ? [
              {
                location: sample.position,
                source_sequence: sample.source_sequence,
                observed_at: sample.observed_at,
                gap_before: sample.gap_before,
              },
            ]
          : [],
      ),
    },
  },
};

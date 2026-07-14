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
};

export const simulatorRecordingPreview: SimulatorRecordingView = {
  preferences: { retention_days: 30 },
  sessions: [previewSession],
};

export const simulatorRecordingSessionPreview: SimulatorSessionView = {
  session: previewSession,
  sample_window_limit: 600,
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

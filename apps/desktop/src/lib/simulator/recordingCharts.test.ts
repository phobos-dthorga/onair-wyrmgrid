import { describe, expect, it } from "vitest";
import type { DisplayPreferences } from "$lib/settings/types";
import type { SimulatorSessionView } from "./types";
import { altitudeRecordingChart, speedRecordingChart } from "./recordingCharts";

const preferences: DisplayPreferences = {
  altitude_unit: "metres",
  speed_unit: "kilometres_per_hour",
  weight_unit: "kilograms",
  fuel_unit: "litres",
  responsive_surfaces: true,
};

const session: SimulatorSessionView = {
  session: {
    id: "session-1",
    provider_id: "provider-1",
    simulator_family: "MSFS_2024",
    aircraft_title: "Cessna 172",
    started_at: "2026-07-15T00:00:00Z",
    status: "active",
    sample_count: 2,
    capture_mode: "manual",
    pinned: false,
    plan_associated: false,
  },
  sample_window_limit: 600,
  sample_window_offset: 0,
  has_older_samples: false,
  has_newer_samples: false,
  events: [],
  samples: [
    {
      source_sequence: 1,
      observed_at: "2026-07-15T00:00:00Z",
      altitude_feet: 1_000,
      indicated_airspeed_knots: 100,
      true_airspeed_knots: 105,
      ground_speed_knots: 95,
      pitch_degrees: 1,
      bank_degrees: 0,
      gap_before: false,
    },
    {
      source_sequence: 4,
      observed_at: "2026-07-15T00:00:04Z",
      altitude_feet: 2_000,
      indicated_airspeed_knots: 110,
      true_airspeed_knots: 115,
      ground_speed_knots: 100,
      pitch_degrees: 2,
      bank_degrees: 3,
      gap_before: true,
    },
  ],
};

describe("simulator recording charts", () => {
  it("converts canonical altitude and preserves gap markers", () => {
    const chart = altitudeRecordingChart(session, preferences);
    expect(chart.unit).toBe("m");
    expect(chart.series[0].points[0].value).toBeCloseTo(304.8);
    expect(chart.series[0].points[1].gap_before).toBe(true);
  });

  it("builds aligned indicated, true, and ground speed series", () => {
    const chart = speedRecordingChart(session, preferences);
    expect(chart.unit).toBe("km/h");
    expect(chart.series.map((series) => series.id)).toEqual([
      "indicated",
      "true",
      "ground",
    ]);
    expect(chart.series.every((series) => series.points.length === 2)).toBe(
      true,
    );
    expect(chart.series[2].points[0].value).toBeCloseTo(175.94);
  });
});

import type { ChartSpec } from "$lib/charts/types";
import type { DisplayPreferences } from "$lib/settings/types";
import { presentAltitude, presentSpeed } from "$lib/settings/units";
import type { SimulatorSessionView } from "./types";

function category(observedAt: string): string {
  const parsed = new Date(observedAt);
  return Number.isNaN(parsed.getTime())
    ? observedAt
    : parsed.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
}

function observedAt(session: SimulatorSessionView): string {
  return session.samples.at(-1)?.observed_at ?? session.session.started_at;
}

export function altitudeRecordingChart(
  session: SimulatorSessionView,
  preferences: DisplayPreferences,
): ChartSpec {
  const presented = presentAltitude(
    session.samples[0]?.altitude_feet,
    preferences.altitude_unit,
  );
  return {
    schema_version: 1,
    id: `simulator-altitude-${session.session.id}`,
    title: "Altitude trace",
    description: `Latest ${session.sample_window_limit.toLocaleString()} exact samples at most. Gaps are shown rather than interpolated.`,
    kind: "area",
    category_axis_label: "Observed time",
    value_axis_label: "Altitude",
    unit: presented.unit,
    series: [
      {
        id: "altitude",
        label: "Altitude",
        points: session.samples.map((sample) => ({
          category: category(sample.observed_at),
          value:
            presentAltitude(sample.altitude_feet, preferences.altitude_unit)
              .value ?? sample.altitude_feet,
          gap_before: sample.gap_before,
        })),
      },
    ],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording",
      observed_at: observedAt(session),
    },
  };
}

export function speedRecordingChart(
  session: SimulatorSessionView,
  preferences: DisplayPreferences,
): ChartSpec {
  const presented = presentSpeed(
    session.samples[0]?.ground_speed_knots,
    preferences.speed_unit,
  );
  const points = (
    id: string,
    label: string,
    select: (sample: SimulatorSessionView["samples"][number]) => number,
  ) => ({
    id,
    label,
    points: session.samples.map((sample) => ({
      category: category(sample.observed_at),
      value:
        presentSpeed(select(sample), preferences.speed_unit).value ??
        select(sample),
      gap_before: sample.gap_before,
    })),
  });
  return {
    schema_version: 1,
    id: `simulator-speed-${session.session.id}`,
    title: "Speed trace",
    description:
      "Indicated, true, and ground speed from the selected local recording window.",
    kind: "line",
    category_axis_label: "Observed time",
    value_axis_label: "Speed",
    unit: presented.unit,
    series: [
      points(
        "indicated",
        "Indicated",
        (sample) => sample.indicated_airspeed_knots,
      ),
      points("true", "True", (sample) => sample.true_airspeed_knots),
      points("ground", "Ground", (sample) => sample.ground_speed_knots),
    ],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording",
      observed_at: observedAt(session),
    },
  };
}

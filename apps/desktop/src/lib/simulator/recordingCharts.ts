import type { ChartSpec } from "$lib/charts/types";
import type { DisplayPreferences } from "$lib/settings/types";
import {
  presentAltitude,
  presentSpeed,
  presentWeight,
} from "$lib/settings/units";
import type {
  SimulatorDebriefTrace,
  SimulatorRecordedSample,
  SimulatorSessionDebrief,
  SimulatorSessionView,
} from "./types";

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

function traceObservedAt(
  debrief: SimulatorSessionDebrief,
  trace: SimulatorDebriefTrace,
): string {
  return trace.samples.at(-1)?.observed_at ?? debrief.session.started_at;
}

function debriefDescription(trace: SimulatorDebriefTrace): string {
  const represented = trace.represented_sample_count.toLocaleString();
  const source = trace.source_sample_count.toLocaleString();
  const method =
    trace.method === "exact"
      ? `${represented} exact samples`
      : `${represented} min/max envelope points from ${source} recorded samples`;
  return `${method}. ${trace.gap_count.toLocaleString()} observed gap${trace.gap_count === 1 ? "" : "s"}; gaps are never interpolated.`;
}

function seriesFromTrace(
  trace: SimulatorDebriefTrace,
  id: string,
  label: string,
  select: (sample: SimulatorRecordedSample) => number | undefined,
  present: (value: number | undefined) => number | undefined,
) {
  return {
    id,
    label,
    points: trace.samples.flatMap((sample) => {
      const value = present(select(sample));
      return value === undefined
        ? []
        : [
            {
              category: category(sample.observed_at),
              value,
              gap_before: sample.gap_before,
            },
          ];
    }),
  };
}

function plannedDurationCategory(
  debrief: SimulatorSessionDebrief,
): string | undefined {
  const seconds = debrief.comparison?.planned_enroute_seconds;
  const started = Date.parse(debrief.session.started_at);
  if (seconds === undefined || Number.isNaN(started)) return undefined;
  const target = started + seconds * 1_000;
  const sample = debrief.traces.altitude.samples.find(
    (item) => Date.parse(item.observed_at) >= target,
  );
  return sample ? category(sample.observed_at) : undefined;
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
    description: `Selected window of at most ${session.sample_window_limit.toLocaleString()} exact samples. Gaps are shown rather than interpolated.`,
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

export function altitudeDebriefChart(
  debrief: SimulatorSessionDebrief,
  preferences: DisplayPreferences,
): ChartSpec {
  const trace = debrief.traces.altitude;
  const plannedAltitude = presentAltitude(
    debrief.comparison?.planned_initial_altitude_ft,
    preferences.altitude_unit,
  ).value;
  const plannedDuration = plannedDurationCategory(debrief);
  return {
    schema_version: 1,
    id: `simulator-altitude-debrief-${debrief.session.id}`,
    title: "Whole-flight altitude",
    description: debriefDescription(trace),
    kind: "area",
    category_axis_label: "Observed time",
    value_axis_label: "Altitude",
    unit: presentAltitude(undefined, preferences.altitude_unit).unit,
    reference_lines: [
      ...(plannedAltitude === undefined
        ? []
        : [
            {
              id: "planned-altitude",
              label: "SimBrief initial altitude",
              axis: "value" as const,
              value: plannedAltitude,
            },
          ]),
      ...(plannedDuration === undefined
        ? []
        : [
            {
              id: "planned-duration",
              label: "SimBrief enroute duration",
              axis: "category" as const,
              value: plannedDuration,
            },
          ]),
    ],
    series: [
      seriesFromTrace(
        trace,
        "altitude",
        "Recorded altitude",
        (sample) => sample.altitude_feet,
        (value) => presentAltitude(value, preferences.altitude_unit).value,
      ),
    ],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording with attributed SimBrief references",
      observed_at: traceObservedAt(debrief, trace),
    },
  };
}

export function speedDebriefChart(
  debrief: SimulatorSessionDebrief,
  preferences: DisplayPreferences,
): ChartSpec {
  const trace = debrief.traces.speed;
  const present = (value: number | undefined) =>
    presentSpeed(value, preferences.speed_unit).value;
  return {
    schema_version: 1,
    id: `simulator-speed-debrief-${debrief.session.id}`,
    title: "Whole-flight speed",
    description: debriefDescription(trace),
    kind: "line",
    category_axis_label: "Observed time",
    value_axis_label: "Speed",
    unit: presentSpeed(undefined, preferences.speed_unit).unit,
    series: [
      seriesFromTrace(
        trace,
        "indicated",
        "Indicated",
        (sample) => sample.indicated_airspeed_knots,
        present,
      ),
      seriesFromTrace(
        trace,
        "true",
        "True",
        (sample) => sample.true_airspeed_knots,
        present,
      ),
      seriesFromTrace(
        trace,
        "ground",
        "Ground",
        (sample) => sample.ground_speed_knots,
        present,
      ),
    ],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording",
      observed_at: traceObservedAt(debrief, trace),
    },
  };
}

export function fuelWeightDebriefChart(
  debrief: SimulatorSessionDebrief,
  preferences: DisplayPreferences,
): ChartSpec {
  const trace = debrief.traces.fuel;
  const plannedTakeoff = presentWeight(
    debrief.comparison?.planned_takeoff_fuel_pounds,
    preferences.weight_unit,
  ).value;
  const plannedLanding = presentWeight(
    debrief.comparison?.planned_landing_fuel_pounds,
    preferences.weight_unit,
  ).value;
  return {
    schema_version: 1,
    id: `simulator-fuel-debrief-${debrief.session.id}`,
    title: "Fuel weight",
    description: trace
      ? `${debriefDescription(trace)} WyrmGrid records fuel weight, so liquid-volume units are not inferred without density and volume facts.`
      : "Fuel weight was not reported for this recording. No liquid volume or density is inferred.",
    kind: "area",
    category_axis_label: "Observed time",
    value_axis_label: "Fuel weight",
    unit: presentWeight(undefined, preferences.weight_unit).unit,
    reference_lines: [
      ...(plannedTakeoff === undefined
        ? []
        : [
            {
              id: "planned-takeoff-fuel",
              label: "SimBrief take-off fuel",
              axis: "value" as const,
              value: plannedTakeoff,
            },
          ]),
      ...(plannedLanding === undefined
        ? []
        : [
            {
              id: "planned-landing-fuel",
              label: "SimBrief landing fuel",
              axis: "value" as const,
              value: plannedLanding,
            },
          ]),
    ],
    series: trace
      ? [
          seriesFromTrace(
            trace,
            "fuel-weight",
            "Recorded fuel weight",
            (sample) => sample.fuel_total_weight_pounds,
            (value) => presentWeight(value, preferences.weight_unit).value,
          ),
        ]
      : [],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording with attributed SimBrief references",
      observed_at: trace
        ? traceObservedAt(debrief, trace)
        : (debrief.session.ended_at ?? debrief.session.started_at),
    },
  };
}

export function attitudeDebriefChart(
  debrief: SimulatorSessionDebrief,
): ChartSpec {
  const trace = debrief.traces.attitude;
  return {
    schema_version: 1,
    id: `simulator-attitude-debrief-${debrief.session.id}`,
    title: "Aircraft attitude",
    description: debriefDescription(trace),
    kind: "line",
    category_axis_label: "Observed time",
    value_axis_label: "Angle",
    unit: "°",
    reference_lines: [{ id: "level", label: "Level", axis: "value", value: 0 }],
    series: [
      seriesFromTrace(
        trace,
        "pitch",
        "Pitch",
        (sample) => sample.pitch_degrees,
        (value) => value,
      ),
      seriesFromTrace(
        trace,
        "bank",
        "Bank",
        (sample) => sample.bank_degrees,
        (value) => value,
      ),
    ],
    provenance: {
      kind: "external_calculation",
      source: "Local simulator recording",
      observed_at: traceObservedAt(debrief, trace),
    },
  };
}

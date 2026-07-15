use std::collections::BTreeSet;

use serde::Serialize;
use wyrmgrid_domain::{Coordinates, FlightPlanSnapshot};

use crate::{
    FlightPlanMapView, PlannedActualComparison, SimulatorRecordedSample, SimulatorSessionSummary,
    build_flight_plan_map_view,
};

pub const SIMULATOR_DEBRIEF_SCHEMA_VERSION: u32 = 1;
pub const SIMULATOR_ROUTE_VIEW_SCHEMA_VERSION: u32 = 2;
pub const MAX_DEBRIEF_TRACE_POINTS: usize = 1_200;

type SampleValue = fn(&SimulatorRecordedSample) -> Option<f64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorDownsamplingMethod {
    Exact,
    MinMaxEnvelope,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorDebriefTrace {
    pub source_sample_count: u64,
    pub represented_sample_count: u64,
    pub gap_count: u64,
    pub method: SimulatorDownsamplingMethod,
    pub samples: Vec<SimulatorRecordedSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorDebriefTraces {
    pub altitude: SimulatorDebriefTrace,
    pub speed: SimulatorDebriefTrace,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel: Option<SimulatorDebriefTrace>,
    pub attitude: SimulatorDebriefTrace,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorRoutePoint {
    pub location: Coordinates,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    pub gap_before: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorRecordedRouteView {
    pub source_sample_count: u64,
    pub represented_point_count: u64,
    pub method: SimulatorDownsamplingMethod,
    pub points: Vec<SimulatorRoutePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorRouteComparisonView {
    pub schema_version: u32,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned: Option<FlightPlanMapView>,
    pub recorded: SimulatorRecordedRouteView,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorSessionDebrief {
    pub schema_version: u32,
    pub session: SimulatorSessionSummary,
    pub source_sample_count: u64,
    pub traces: SimulatorDebriefTraces,
    pub route: SimulatorRouteComparisonView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<PlannedActualComparison>,
}

pub(crate) fn build_simulator_debrief(
    session: SimulatorSessionSummary,
    samples: &[SimulatorRecordedSample],
    plan: Option<&FlightPlanSnapshot>,
    comparison: Option<PlannedActualComparison>,
) -> SimulatorSessionDebrief {
    let fuel_samples =
        samples_with_value(samples, |sample| sample.fuel_total_weight_pounds.is_some());
    SimulatorSessionDebrief {
        schema_version: SIMULATOR_DEBRIEF_SCHEMA_VERSION,
        source_sample_count: samples.len() as u64,
        traces: SimulatorDebriefTraces {
            altitude: downsample_trace(samples, &[altitude]),
            speed: downsample_trace(samples, &[indicated_airspeed, true_airspeed, ground_speed]),
            fuel: (!fuel_samples.is_empty())
                .then(|| downsample_trace(&fuel_samples, &[fuel_weight])),
            attitude: downsample_trace(samples, &[pitch, bank]),
        },
        route: route_comparison(&session.id, samples, plan),
        session,
        comparison,
    }
}

fn downsample_trace(
    samples: &[SimulatorRecordedSample],
    selectors: &[SampleValue],
) -> SimulatorDebriefTrace {
    let source_sample_count = samples.len() as u64;
    let gap_count = samples.iter().filter(|sample| sample.gap_before).count() as u64;
    if samples.len() <= MAX_DEBRIEF_TRACE_POINTS || selectors.is_empty() {
        return SimulatorDebriefTrace {
            source_sample_count,
            represented_sample_count: source_sample_count,
            gap_count,
            method: SimulatorDownsamplingMethod::Exact,
            samples: samples.to_vec(),
        };
    }

    let points_per_bucket = selectors.len().saturating_mul(2).max(1);
    let bucket_count = (MAX_DEBRIEF_TRACE_POINTS.saturating_sub(2) / points_per_bucket).max(1);
    let interior_count = samples.len().saturating_sub(2);
    let bucket_width = interior_count.div_ceil(bucket_count);
    let mut selected = BTreeSet::from([0, samples.len() - 1]);

    for bucket_start in (1..samples.len() - 1).step_by(bucket_width.max(1)) {
        let bucket_end = (bucket_start + bucket_width).min(samples.len() - 1);
        for selector in selectors {
            let mut minimum: Option<(usize, f64)> = None;
            let mut maximum: Option<(usize, f64)> = None;
            for (index, sample) in samples
                .iter()
                .enumerate()
                .take(bucket_end)
                .skip(bucket_start)
            {
                let Some(value) = selector(sample).filter(|value| value.is_finite()) else {
                    continue;
                };
                if minimum.is_none_or(|(_, current)| value < current) {
                    minimum = Some((index, value));
                }
                if maximum.is_none_or(|(_, current)| value > current) {
                    maximum = Some((index, value));
                }
            }
            if let Some((index, _)) = minimum {
                selected.insert(index);
            }
            if let Some((index, _)) = maximum {
                selected.insert(index);
            }
        }
    }

    let mut previous_index = None;
    let downsampled = selected
        .into_iter()
        .map(|index| {
            let mut sample = samples[index].clone();
            if let Some(previous) = previous_index {
                sample.gap_before = samples[previous + 1..=index]
                    .iter()
                    .any(|candidate| candidate.gap_before);
            }
            previous_index = Some(index);
            sample
        })
        .collect::<Vec<_>>();

    SimulatorDebriefTrace {
        source_sample_count,
        represented_sample_count: downsampled.len() as u64,
        gap_count,
        method: SimulatorDownsamplingMethod::MinMaxEnvelope,
        samples: downsampled,
    }
}

fn samples_with_value(
    samples: &[SimulatorRecordedSample],
    available: impl Fn(&SimulatorRecordedSample) -> bool,
) -> Vec<SimulatorRecordedSample> {
    let mut gap_pending = false;
    let mut filtered = Vec::new();
    for sample in samples {
        gap_pending |= sample.gap_before;
        if available(sample) {
            let mut included = sample.clone();
            included.gap_before |= gap_pending;
            gap_pending = false;
            filtered.push(included);
        } else {
            gap_pending = true;
        }
    }
    filtered
}

fn route_comparison(
    session_id: &str,
    samples: &[SimulatorRecordedSample],
    plan: Option<&FlightPlanSnapshot>,
) -> SimulatorRouteComparisonView {
    let positioned_samples = samples_with_value(samples, |sample| sample.position.is_some());
    let route_trace = downsample_trace(&positioned_samples, &[latitude, longitude]);
    let recorded_points = route_trace
        .samples
        .iter()
        .filter_map(|sample| {
            sample.position.map(|location| SimulatorRoutePoint {
                location,
                label: None,
                source_sequence: Some(sample.source_sequence),
                observed_at: Some(sample.observed_at.clone()),
                gap_before: sample.gap_before,
            })
        })
        .collect::<Vec<_>>();
    SimulatorRouteComparisonView {
        schema_version: SIMULATOR_ROUTE_VIEW_SCHEMA_VERSION,
        session_id: session_id.to_owned(),
        planned: plan.map(build_flight_plan_map_view),
        recorded: SimulatorRecordedRouteView {
            source_sample_count: positioned_samples.len() as u64,
            represented_point_count: recorded_points.len() as u64,
            method: route_trace.method,
            points: recorded_points,
        },
    }
}

fn altitude(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.altitude_feet)
}

fn indicated_airspeed(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.indicated_airspeed_knots)
}

fn true_airspeed(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.true_airspeed_knots)
}

fn ground_speed(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.ground_speed_knots)
}

fn fuel_weight(sample: &SimulatorRecordedSample) -> Option<f64> {
    sample.fuel_total_weight_pounds
}

fn pitch(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.pitch_degrees)
}

fn bank(sample: &SimulatorRecordedSample) -> Option<f64> {
    Some(sample.bank_degrees)
}

fn latitude(sample: &SimulatorRecordedSample) -> Option<f64> {
    sample.position.map(|position| position.latitude)
}

fn longitude(sample: &SimulatorRecordedSample) -> Option<f64> {
    sample.position.map(|position| position.longitude)
}

#[cfg(test)]
#[path = "tests/simulator_debrief.rs"]
mod tests;

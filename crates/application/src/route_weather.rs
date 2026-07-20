use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use wyrmgrid_domain::{
    Coordinates, GlobalWeatherGridPoint, GlobalWeatherLayerData, GlobalWeatherLayerSnapshot,
    GlobalWeatherTimeScope, OperationalProvenance, PlannedSchedule, WeatherCondition,
};
use wyrmgrid_plugin_protocol::WeatherTimeWindow;

use crate::FlightPlanMapView;

pub const ROUTE_WEATHER_ANALYSIS_SCHEMA_VERSION: u32 = 3;
pub const ROUTE_WEATHER_SAMPLE_INTERVAL_NM: f64 = 300.0;
pub const ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM: f64 = 1_200.0;
pub const ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS: i64 = 3 * 60 * 60;
pub const MAX_ROUTE_WEATHER_SAMPLES: usize = 64;

const EARTH_RADIUS_NM: f64 = 3_440.065;
const MAX_ROUTE_DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;
const HISTORICAL_WEATHER_WINDOW_BUFFER_SECONDS: i64 = 3 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherTemporalMode {
    Live,
    Historical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherAvailability {
    Ready,
    Partial,
    RouteUnavailable,
    SourceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherTimingAvailability {
    Ready,
    DepartureUnavailable,
    DurationUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherDepartureBasis {
    ScheduledOff,
    ScheduledOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherDurationBasis {
    EstimatedEnroute,
    ScheduledOn,
    ScheduledIn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteWeatherTiming {
    pub availability: RouteWeatherTimingAvailability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_basis: Option<RouteWeatherDepartureBasis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_basis: Option<RouteWeatherDurationBasis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departure_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherTemporalSupport {
    EtaMatched,
    CurrentContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherRadarRelationship {
    ObservationOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherRadarContext {
    pub layer_id: String,
    pub title: String,
    pub provenance: OperationalProvenance,
    pub frame_time: DateTime<Utc>,
    pub relationship: RouteWeatherRadarRelationship,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherSourceSample {
    pub point_id: String,
    pub location: Coordinates,
    pub support_distance_nm: f64,
    pub temporal_support: RouteWeatherTemporalSupport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_offset_seconds: Option<i64>,
    pub condition: WeatherCondition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precipitation_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_cover_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_direction_degrees: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_speed_kt: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherSample {
    pub id: String,
    pub segment_index: u32,
    pub distance_from_origin_nm: f64,
    pub location: Coordinates,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_arrival_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RouteWeatherSourceSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherLayerAnalysis {
    pub layer_id: String,
    pub title: String,
    pub provenance: OperationalProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_scope: Option<GlobalWeatherTimeScope>,
    pub availability: RouteWeatherAvailability,
    pub samples: Vec<RouteWeatherSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherAnalysis {
    pub schema_version: u32,
    pub plan_id: String,
    pub sample_interval_nm: f64,
    pub maximum_support_distance_nm: f64,
    pub maximum_temporal_support_seconds: i64,
    pub mapped_route_point_count: usize,
    pub unresolved_route_point_count: usize,
    pub timing: RouteWeatherTiming,
    pub temporal_mode: RouteWeatherTemporalMode,
    pub availability: RouteWeatherAvailability,
    pub layers: Vec<RouteWeatherLayerAnalysis>,
    pub radar_contexts: Vec<RouteWeatherRadarContext>,
}

#[derive(Clone, Copy)]
struct RouteCheckpoint {
    segment_index: u32,
    distance_from_origin_nm: f64,
    location: Coordinates,
}

pub(crate) fn build_route_weather_analysis(
    plan: &FlightPlanMapView,
    schedule: Option<&PlannedSchedule>,
    layers: &[GlobalWeatherLayerSnapshot],
) -> RouteWeatherAnalysis {
    let route_points = plan
        .points
        .iter()
        .filter(|point| point.on_route)
        .collect::<Vec<_>>();
    let mapped_route_point_count = route_points
        .iter()
        .filter(|point| point.location.is_some())
        .count();
    let unresolved_route_point_count = route_points.len() - mapped_route_point_count;
    let checkpoints = route_checkpoints(&route_points);
    let total_route_distance_nm = route_total_distance_nm(&route_points);
    let timing = route_timing(schedule);
    let temporal_mode = route_weather_temporal_mode(schedule, Utc::now());
    let radar_contexts = route_radar_contexts(layers);
    if checkpoints.is_empty() {
        return RouteWeatherAnalysis {
            schema_version: ROUTE_WEATHER_ANALYSIS_SCHEMA_VERSION,
            plan_id: plan.plan_id.clone(),
            sample_interval_nm: ROUTE_WEATHER_SAMPLE_INTERVAL_NM,
            maximum_support_distance_nm: ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM,
            maximum_temporal_support_seconds: ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS,
            mapped_route_point_count,
            unresolved_route_point_count,
            timing,
            temporal_mode,
            availability: RouteWeatherAvailability::RouteUnavailable,
            layers: Vec::new(),
            radar_contexts,
        };
    }

    let mut analyses = layers
        .iter()
        .filter_map(|layer| {
            let GlobalWeatherLayerData::Grid { points } = &layer.data else {
                return None;
            };
            let samples = checkpoints
                .iter()
                .enumerate()
                .map(|(index, checkpoint)| {
                    let estimated_arrival_at = checkpoint_eta(
                        checkpoint.distance_from_origin_nm,
                        total_route_distance_nm,
                        &timing,
                    );
                    RouteWeatherSample {
                        id: format!("route-weather:{index:04}"),
                        segment_index: checkpoint.segment_index,
                        distance_from_origin_nm: rounded(checkpoint.distance_from_origin_nm),
                        location: checkpoint.location,
                        estimated_arrival_at,
                        source: nearest_supported_point(
                            checkpoint.location,
                            estimated_arrival_at,
                            layer.provenance.retrieved_at,
                            points,
                        ),
                    }
                })
                .collect::<Vec<_>>();
            let supported = samples
                .iter()
                .filter(|sample| sample.source.is_some())
                .count();
            let eta_matched = samples.iter().filter(|sample| {
                sample.source.as_ref().is_some_and(|source| {
                    source.temporal_support == RouteWeatherTemporalSupport::EtaMatched
                })
            });
            let availability = if supported == samples.len()
                && eta_matched.count() == samples.len()
                && timing.availability == RouteWeatherTimingAvailability::Ready
            {
                RouteWeatherAvailability::Ready
            } else {
                RouteWeatherAvailability::Partial
            };
            Some(RouteWeatherLayerAnalysis {
                layer_id: layer.id.clone(),
                title: layer.title.clone(),
                provenance: layer.provenance.clone(),
                time_scope: layer.time_scope.clone(),
                availability,
                samples,
            })
        })
        .collect::<Vec<_>>();
    analyses.sort_by(|left, right| {
        left.provenance
            .provider
            .cmp(&right.provenance.provider)
            .then_with(|| left.layer_id.cmp(&right.layer_id))
    });
    let availability = if analyses.is_empty() {
        RouteWeatherAvailability::SourceUnavailable
    } else if analyses
        .iter()
        .all(|analysis| analysis.availability == RouteWeatherAvailability::Ready)
    {
        RouteWeatherAvailability::Ready
    } else {
        RouteWeatherAvailability::Partial
    };

    RouteWeatherAnalysis {
        schema_version: ROUTE_WEATHER_ANALYSIS_SCHEMA_VERSION,
        plan_id: plan.plan_id.clone(),
        sample_interval_nm: ROUTE_WEATHER_SAMPLE_INTERVAL_NM,
        maximum_support_distance_nm: ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM,
        maximum_temporal_support_seconds: ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS,
        mapped_route_point_count,
        unresolved_route_point_count,
        timing,
        temporal_mode,
        availability,
        layers: analyses,
        radar_contexts,
    }
}

pub fn historical_weather_window(
    schedule: Option<&PlannedSchedule>,
    now: DateTime<Utc>,
) -> Option<WeatherTimeWindow> {
    if route_weather_temporal_mode(schedule, now) != RouteWeatherTemporalMode::Historical {
        return None;
    }
    let timing = route_timing(schedule);
    let departure = timing.departure_at?;
    let duration_seconds = i64::from(timing.duration_seconds?);
    let arrival = departure.checked_add_signed(Duration::seconds(duration_seconds))?;
    let starts_at = departure
        .checked_sub_signed(Duration::seconds(HISTORICAL_WEATHER_WINDOW_BUFFER_SECONDS))?;
    let ends_at =
        arrival.checked_add_signed(Duration::seconds(HISTORICAL_WEATHER_WINDOW_BUFFER_SECONDS))?;
    let target_at = departure.checked_add_signed(Duration::seconds(duration_seconds / 2))?;
    let window = WeatherTimeWindow {
        target_at,
        starts_at,
        ends_at,
    };
    window.is_valid().then_some(window)
}

pub fn route_weather_temporal_mode(
    schedule: Option<&PlannedSchedule>,
    now: DateTime<Utc>,
) -> RouteWeatherTemporalMode {
    let timing = route_timing(schedule);
    let Some((departure, duration_seconds)) = timing.departure_at.zip(timing.duration_seconds)
    else {
        return RouteWeatherTemporalMode::Live;
    };
    let historical = departure
        .checked_add_signed(Duration::seconds(i64::from(duration_seconds)))
        .and_then(|arrival| {
            arrival.checked_add_signed(Duration::seconds(
                ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS,
            ))
        })
        .is_some_and(|support_ends_at| support_ends_at < now);
    if historical {
        RouteWeatherTemporalMode::Historical
    } else {
        RouteWeatherTemporalMode::Live
    }
}

fn route_timing(schedule: Option<&PlannedSchedule>) -> RouteWeatherTiming {
    let departure = schedule.and_then(|schedule| {
        schedule
            .scheduled_off
            .map(|at| (at, RouteWeatherDepartureBasis::ScheduledOff))
            .or_else(|| {
                schedule
                    .scheduled_out
                    .map(|at| (at, RouteWeatherDepartureBasis::ScheduledOut))
            })
    });
    let duration = schedule.and_then(|schedule| {
        schedule
            .estimated_enroute_seconds
            .filter(|seconds| *seconds > 0 && i64::from(*seconds) <= MAX_ROUTE_DURATION_SECONDS)
            .map(|seconds| (seconds, RouteWeatherDurationBasis::EstimatedEnroute))
            .or_else(|| {
                departure.and_then(|(departure_at, _)| {
                    schedule
                        .scheduled_on
                        .and_then(|arrival| schedule_duration(departure_at, arrival))
                        .map(|seconds| (seconds, RouteWeatherDurationBasis::ScheduledOn))
                        .or_else(|| {
                            schedule
                                .scheduled_in
                                .and_then(|arrival| schedule_duration(departure_at, arrival))
                                .map(|seconds| (seconds, RouteWeatherDurationBasis::ScheduledIn))
                        })
                })
            })
    });
    let availability = if departure.is_none() {
        RouteWeatherTimingAvailability::DepartureUnavailable
    } else if duration.is_none() {
        RouteWeatherTimingAvailability::DurationUnavailable
    } else {
        RouteWeatherTimingAvailability::Ready
    };
    RouteWeatherTiming {
        availability,
        departure_basis: departure.map(|(_, basis)| basis),
        duration_basis: duration.map(|(_, basis)| basis),
        departure_at: departure.map(|(at, _)| at),
        duration_seconds: duration.map(|(seconds, _)| seconds),
    }
}

fn schedule_duration(departure: DateTime<Utc>, arrival: DateTime<Utc>) -> Option<u32> {
    let seconds = arrival.signed_duration_since(departure).num_seconds();
    (seconds > 0 && seconds <= MAX_ROUTE_DURATION_SECONDS).then_some(seconds as u32)
}

fn checkpoint_eta(
    distance_from_origin_nm: f64,
    total_route_distance_nm: f64,
    timing: &RouteWeatherTiming,
) -> Option<DateTime<Utc>> {
    let departure = timing.departure_at?;
    let duration = timing.duration_seconds?;
    if total_route_distance_nm <= 0.0 {
        return None;
    }
    let fraction = (distance_from_origin_nm / total_route_distance_nm).clamp(0.0, 1.0);
    departure.checked_add_signed(Duration::seconds(
        (f64::from(duration) * fraction).round() as i64
    ))
}

fn route_radar_contexts(layers: &[GlobalWeatherLayerSnapshot]) -> Vec<RouteWeatherRadarContext> {
    let mut contexts = std::collections::BTreeMap::new();
    for layer in layers {
        let GlobalWeatherLayerData::RasterTiles { frame_time, .. } = &layer.data else {
            continue;
        };
        let key = (layer.provenance.provider.clone(), layer.id.clone());
        let candidate = RouteWeatherRadarContext {
            layer_id: layer.id.clone(),
            title: layer.title.clone(),
            provenance: layer.provenance.clone(),
            frame_time: *frame_time,
            relationship: RouteWeatherRadarRelationship::ObservationOnly,
        };
        if contexts
            .get(&key)
            .is_none_or(|current: &RouteWeatherRadarContext| {
                candidate.frame_time > current.frame_time
            })
        {
            contexts.insert(key, candidate);
        }
    }
    contexts.into_values().collect()
}

fn route_checkpoints(route_points: &[&crate::FlightPlanMapPoint]) -> Vec<RouteCheckpoint> {
    let mut checkpoints = Vec::new();
    let mut distance_from_origin_nm = 0.0;
    let mut previous = None;
    let mut segment_index = 0_u32;
    let mut gap_pending = false;
    for point in route_points {
        let Some(location) = point.location else {
            previous = None;
            gap_pending = true;
            continue;
        };
        if point.gap_before {
            previous = None;
            gap_pending = true;
        }
        let Some(start) = previous else {
            if gap_pending && !checkpoints.is_empty() {
                segment_index = segment_index.saturating_add(1);
            }
            gap_pending = false;
            previous = Some(location);
            continue;
        };
        let segment_distance = great_circle_distance_nm(start, location);
        if !segment_distance.is_finite() || segment_distance <= 0.0 {
            previous = Some(location);
            continue;
        }
        let steps = (segment_distance / ROUTE_WEATHER_SAMPLE_INTERVAL_NM)
            .ceil()
            .max(1.0) as usize;
        for step in 0..=steps {
            if checkpoints.len() >= MAX_ROUTE_WEATHER_SAMPLES {
                return checkpoints;
            }
            if step == 0
                && checkpoints
                    .last()
                    .is_some_and(|checkpoint: &RouteCheckpoint| checkpoint.location == start)
            {
                continue;
            }
            let fraction = step as f64 / steps as f64;
            checkpoints.push(RouteCheckpoint {
                segment_index,
                distance_from_origin_nm: distance_from_origin_nm + segment_distance * fraction,
                location: great_circle_interpolate(start, location, fraction),
            });
        }
        distance_from_origin_nm += segment_distance;
        previous = Some(location);
    }
    checkpoints
}

fn route_total_distance_nm(route_points: &[&crate::FlightPlanMapPoint]) -> f64 {
    let mut total = 0.0;
    let mut previous = None;
    for point in route_points {
        let Some(location) = point.location else {
            previous = None;
            continue;
        };
        if point.gap_before {
            previous = None;
        }
        if let Some(start) = previous {
            let distance = great_circle_distance_nm(start, location);
            if distance.is_finite() && distance > 0.0 {
                total += distance;
            }
        }
        previous = Some(location);
    }
    total
}

fn nearest_supported_point(
    location: Coordinates,
    estimated_arrival_at: Option<DateTime<Utc>>,
    retrieved_at: DateTime<Utc>,
    points: &[GlobalWeatherGridPoint],
) -> Option<RouteWeatherSourceSample> {
    if let Some(eta) = estimated_arrival_at {
        if let Some((point, distance, offset)) = points
            .iter()
            .filter_map(|point| {
                let valid_at = point.valid_at?;
                let distance = great_circle_distance_nm(location, point.location);
                let offset = valid_at.signed_duration_since(eta).num_seconds();
                (distance.is_finite()
                    && distance <= ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM
                    && offset.unsigned_abs() <= ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS as u64)
                    .then_some((point, distance, offset))
            })
            .min_by(|left, right| {
                left.2
                    .unsigned_abs()
                    .cmp(&right.2.unsigned_abs())
                    .then_with(|| left.1.total_cmp(&right.1))
                    .then_with(|| left.0.id.cmp(&right.0.id))
            })
        {
            return Some(source_sample(
                point,
                distance,
                RouteWeatherTemporalSupport::EtaMatched,
                Some(offset),
            ));
        }

        return nearest_context_point(location, points, true, retrieved_at);
    }

    nearest_context_point(location, points, false, retrieved_at)
}

fn nearest_context_point(
    location: Coordinates,
    points: &[GlobalWeatherGridPoint],
    legacy_only: bool,
    retrieved_at: DateTime<Utc>,
) -> Option<RouteWeatherSourceSample> {
    let (point, distance) = points
        .iter()
        .filter(|point| !legacy_only || point.valid_at.is_none())
        .filter_map(|point| {
            let distance = great_circle_distance_nm(location, point.location);
            (distance.is_finite() && distance <= ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM)
                .then_some((point, distance))
        })
        .min_by(|left, right| {
            context_time_distance(left.0, retrieved_at)
                .cmp(&context_time_distance(right.0, retrieved_at))
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.0.id.cmp(&right.0.id))
        })?;
    Some(source_sample(
        point,
        distance,
        RouteWeatherTemporalSupport::CurrentContext,
        None,
    ))
}

fn context_time_distance(point: &GlobalWeatherGridPoint, retrieved_at: DateTime<Utc>) -> u64 {
    point
        .valid_at
        .map(|valid_at| {
            valid_at
                .signed_duration_since(retrieved_at)
                .num_seconds()
                .unsigned_abs()
        })
        .unwrap_or(u64::MAX)
}

fn source_sample(
    point: &GlobalWeatherGridPoint,
    distance: f64,
    temporal_support: RouteWeatherTemporalSupport,
    time_offset_seconds: Option<i64>,
) -> RouteWeatherSourceSample {
    RouteWeatherSourceSample {
        point_id: point.id.clone(),
        location: point.location,
        support_distance_nm: rounded(distance),
        temporal_support,
        valid_at: point.valid_at,
        time_offset_seconds,
        condition: point.condition,
        temperature_c: point.temperature_c,
        precipitation_mm: point.precipitation_mm,
        cloud_cover_percent: point.cloud_cover_percent,
        wind_direction_degrees: point.wind_direction_degrees,
        wind_speed_kt: point.wind_speed_kt,
    }
}

fn great_circle_distance_nm(left: Coordinates, right: Coordinates) -> f64 {
    let left_latitude = left.latitude.to_radians();
    let right_latitude = right.latitude.to_radians();
    let latitude_delta = right_latitude - left_latitude;
    let longitude_delta = (right.longitude - left.longitude).to_radians();
    let haversine = (latitude_delta / 2.0).sin().powi(2)
        + left_latitude.cos() * right_latitude.cos() * (longitude_delta / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_NM * haversine.sqrt().asin()
}

fn great_circle_interpolate(start: Coordinates, end: Coordinates, fraction: f64) -> Coordinates {
    let start_vector = coordinate_vector(start);
    let end_vector = coordinate_vector(end);
    let angle = start_vector
        .iter()
        .zip(end_vector)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .clamp(-1.0, 1.0)
        .acos();
    if angle.abs() < f64::EPSILON {
        return start;
    }
    let denominator = angle.sin();
    let start_weight = ((1.0 - fraction) * angle).sin() / denominator;
    let end_weight = (fraction * angle).sin() / denominator;
    let vector = [
        start_weight * start_vector[0] + end_weight * end_vector[0],
        start_weight * start_vector[1] + end_weight * end_vector[1],
        start_weight * start_vector[2] + end_weight * end_vector[2],
    ];
    Coordinates {
        latitude: vector[2]
            .atan2((vector[0].powi(2) + vector[1].powi(2)).sqrt())
            .to_degrees(),
        longitude: vector[1].atan2(vector[0]).to_degrees(),
    }
}

fn coordinate_vector(coordinate: Coordinates) -> [f64; 3] {
    let latitude = coordinate.latitude.to_radians();
    let longitude = coordinate.longitude.to_radians();
    [
        latitude.cos() * longitude.cos(),
        latitude.cos() * longitude.sin(),
        latitude.sin(),
    ]
}

fn rounded(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
#[path = "tests/route_weather.rs"]
mod tests;

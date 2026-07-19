use serde::Serialize;
use wyrmgrid_domain::{
    Coordinates, GlobalWeatherGridPoint, GlobalWeatherLayerData, GlobalWeatherLayerSnapshot,
    OperationalProvenance, WeatherCondition,
};

use crate::FlightPlanMapView;

pub const ROUTE_WEATHER_ANALYSIS_SCHEMA_VERSION: u32 = 1;
pub const ROUTE_WEATHER_SAMPLE_INTERVAL_NM: f64 = 300.0;
pub const ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM: f64 = 1_200.0;
pub const MAX_ROUTE_WEATHER_SAMPLES: usize = 64;

const EARTH_RADIUS_NM: f64 = 3_440.065;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWeatherAvailability {
    Ready,
    Partial,
    RouteUnavailable,
    SourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherSourceSample {
    pub point_id: String,
    pub location: Coordinates,
    pub support_distance_nm: f64,
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
    pub source: Option<RouteWeatherSourceSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherLayerAnalysis {
    pub layer_id: String,
    pub title: String,
    pub provenance: OperationalProvenance,
    pub availability: RouteWeatherAvailability,
    pub samples: Vec<RouteWeatherSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteWeatherAnalysis {
    pub schema_version: u32,
    pub plan_id: String,
    pub sample_interval_nm: f64,
    pub maximum_support_distance_nm: f64,
    pub mapped_route_point_count: usize,
    pub unresolved_route_point_count: usize,
    pub availability: RouteWeatherAvailability,
    pub layers: Vec<RouteWeatherLayerAnalysis>,
}

#[derive(Clone, Copy)]
struct RouteCheckpoint {
    segment_index: u32,
    distance_from_origin_nm: f64,
    location: Coordinates,
}

pub(crate) fn build_route_weather_analysis(
    plan: &FlightPlanMapView,
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
    if checkpoints.is_empty() {
        return RouteWeatherAnalysis {
            schema_version: ROUTE_WEATHER_ANALYSIS_SCHEMA_VERSION,
            plan_id: plan.plan_id.clone(),
            sample_interval_nm: ROUTE_WEATHER_SAMPLE_INTERVAL_NM,
            maximum_support_distance_nm: ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM,
            mapped_route_point_count,
            unresolved_route_point_count,
            availability: RouteWeatherAvailability::RouteUnavailable,
            layers: Vec::new(),
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
                .map(|(index, checkpoint)| RouteWeatherSample {
                    id: format!("route-weather:{index:04}"),
                    segment_index: checkpoint.segment_index,
                    distance_from_origin_nm: rounded(checkpoint.distance_from_origin_nm),
                    location: checkpoint.location,
                    source: nearest_supported_point(checkpoint.location, points),
                })
                .collect::<Vec<_>>();
            let supported = samples
                .iter()
                .filter(|sample| sample.source.is_some())
                .count();
            let availability = if supported == samples.len() {
                RouteWeatherAvailability::Ready
            } else {
                RouteWeatherAvailability::Partial
            };
            Some(RouteWeatherLayerAnalysis {
                layer_id: layer.id.clone(),
                title: layer.title.clone(),
                provenance: layer.provenance.clone(),
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
        mapped_route_point_count,
        unresolved_route_point_count,
        availability,
        layers: analyses,
    }
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

fn nearest_supported_point(
    location: Coordinates,
    points: &[GlobalWeatherGridPoint],
) -> Option<RouteWeatherSourceSample> {
    let (point, distance) = points
        .iter()
        .filter_map(|point| {
            let distance = great_circle_distance_nm(location, point.location);
            distance.is_finite().then_some((point, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))?;
    (distance <= ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM).then(|| RouteWeatherSourceSample {
        point_id: point.id.clone(),
        location: point.location,
        support_distance_nm: rounded(distance),
        condition: point.condition,
        temperature_c: point.temperature_c,
        precipitation_mm: point.precipitation_mm,
        cloud_cover_percent: point.cloud_cover_percent,
        wind_direction_degrees: point.wind_direction_degrees,
        wind_speed_kt: point.wind_speed_kt,
    })
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

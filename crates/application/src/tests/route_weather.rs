use chrono::{Duration, TimeZone, Utc};
use wyrmgrid_domain::{
    GlobalWeatherGridPoint, GlobalWeatherLayerData, GlobalWeatherLayerSnapshot,
    OperationalProvenance, PlannedSchedule, ProvenanceKind, SnapshotFreshness, WeatherCondition,
};

use super::*;

fn plan() -> FlightPlanMapView {
    FlightPlanMapView {
        schema_version: crate::FLIGHT_PLAN_MAP_SCHEMA_VERSION,
        plan_id: "plan-route-weather".into(),
        origin_icao: "YSSY".into(),
        destination_icao: "NZAA".into(),
        airac: Some("2607".into()),
        source_text: None,
        provenance: provenance(ProvenanceKind::ExternalFact),
        points: vec![
            route_point("origin", -33.95, 151.18, false),
            route_point("mid", -35.0, 165.0, false),
            route_point("destination", -37.01, 174.79, false),
        ],
    }
}

fn route_point(
    id: &str,
    latitude: f64,
    longitude: f64,
    gap_before: bool,
) -> crate::FlightPlanMapPoint {
    crate::FlightPlanMapPoint {
        id: id.into(),
        kind: crate::FlightPlanMapPointKind::RouteLeg,
        label: id.into(),
        sequence: None,
        airway: None,
        location: Some(Coordinates {
            latitude,
            longitude,
        }),
        on_route: true,
        gap_before,
    }
}

fn provenance(kind: ProvenanceKind) -> OperationalProvenance {
    let at = Utc.with_ymd_and_hms(2026, 7, 19, 4, 0, 0).unwrap();
    OperationalProvenance {
        kind,
        provider: "open-meteo.com".into(),
        provider_revision: Some("forecast-api-v1".into()),
        generated_at: Some(at),
        retrieved_at: at,
        transformation_version: 1,
        freshness: SnapshotFreshness::Current,
    }
}

fn grid(points: Vec<GlobalWeatherGridPoint>) -> GlobalWeatherLayerSnapshot {
    GlobalWeatherLayerSnapshot {
        schema_version: wyrmgrid_domain::GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "open-meteo-global".into(),
        title: "Global model weather".into(),
        data: GlobalWeatherLayerData::Grid { points },
        provenance: provenance(ProvenanceKind::ExternalCalculation),
    }
}

fn grid_point(
    id: &str,
    latitude: f64,
    longitude: f64,
    condition: WeatherCondition,
) -> GlobalWeatherGridPoint {
    GlobalWeatherGridPoint {
        id: id.into(),
        location: Coordinates {
            latitude,
            longitude,
        },
        valid_at: None,
        condition,
        temperature_c: Some(12.0),
        precipitation_mm: Some(1.5),
        cloud_cover_percent: Some(80.0),
        wind_direction_degrees: Some(240.0),
        wind_speed_kt: Some(22.0),
        provider_weather_code: Some(61),
    }
}

fn timed_grid_point(
    id: &str,
    latitude: f64,
    longitude: f64,
    condition: WeatherCondition,
    valid_at: chrono::DateTime<Utc>,
) -> GlobalWeatherGridPoint {
    GlobalWeatherGridPoint {
        valid_at: Some(valid_at),
        ..grid_point(id, latitude, longitude, condition)
    }
}

fn schedule(
    departure: chrono::DateTime<Utc>,
    estimated_enroute_seconds: Option<u32>,
) -> PlannedSchedule {
    PlannedSchedule {
        scheduled_out: Some(departure - Duration::minutes(15)),
        scheduled_off: Some(departure),
        scheduled_on: None,
        scheduled_in: None,
        estimated_enroute_seconds,
    }
}

#[test]
fn samples_resolved_route_segments_and_preserves_source_distance_and_provenance() {
    let analysis = build_route_weather_analysis(
        &plan(),
        None,
        &[grid(vec![
            grid_point("west", -34.0, 151.0, WeatherCondition::Rain),
            grid_point("east", -37.0, 175.0, WeatherCondition::Cloud),
        ])],
    );

    assert_eq!(analysis.availability, RouteWeatherAvailability::Partial);
    assert!(analysis.layers[0].samples.len() > 3);
    assert_eq!(analysis.layers[0].provenance.provider, "open-meteo.com");
    assert!(analysis.layers[0].samples.iter().all(|sample| {
        sample.source.as_ref().is_some_and(|source| {
            source.support_distance_nm <= ROUTE_WEATHER_MAX_SUPPORT_DISTANCE_NM
        })
    }));
}

#[test]
fn refuses_to_bridge_unresolved_route_gaps() {
    let mut plan = plan();
    plan.points[1].location = None;
    plan.points[2].gap_before = true;
    let analysis = build_route_weather_analysis(
        &plan,
        None,
        &[grid(vec![grid_point(
            "source",
            -35.0,
            165.0,
            WeatherCondition::Rain,
        )])],
    );

    assert_eq!(
        analysis.availability,
        RouteWeatherAvailability::RouteUnavailable
    );
    assert_eq!(analysis.unresolved_route_point_count, 1);
    assert!(analysis.layers.is_empty());
}

#[test]
fn keeps_supported_route_sections_on_opposite_sides_of_a_gap_separate() {
    let mut plan = plan();
    plan.points = vec![
        route_point("a", -34.0, 151.0, false),
        route_point("b", -34.5, 154.0, false),
        route_point("c", -36.0, 170.0, true),
        route_point("d", -37.0, 175.0, false),
    ];
    let analysis = build_route_weather_analysis(
        &plan,
        None,
        &[grid(vec![
            grid_point("west", -34.0, 151.0, WeatherCondition::Rain),
            grid_point("east", -37.0, 175.0, WeatherCondition::Cloud),
        ])],
    );
    let segment_indexes = analysis.layers[0]
        .samples
        .iter()
        .map(|sample| sample.segment_index)
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(segment_indexes, std::collections::BTreeSet::from([0, 1]));
}

#[test]
fn keeps_distant_model_support_and_missing_layers_explicitly_unavailable() {
    let analysis = build_route_weather_analysis(
        &plan(),
        None,
        &[grid(vec![grid_point(
            "distant",
            60.0,
            -20.0,
            WeatherCondition::Clear,
        )])],
    );
    assert_eq!(analysis.availability, RouteWeatherAvailability::Partial);
    assert!(
        analysis.layers[0]
            .samples
            .iter()
            .all(|sample| sample.source.is_none())
    );

    let no_source = build_route_weather_analysis(&plan(), None, &[]);
    assert_eq!(
        no_source.availability,
        RouteWeatherAvailability::SourceUnavailable
    );
}

#[test]
fn interpolates_antimeridian_segments_without_crossing_the_long_way() {
    let plan = FlightPlanMapView {
        points: vec![
            route_point("west", 10.0, 179.0, false),
            route_point("east", 10.0, -179.0, false),
        ],
        ..plan()
    };
    let analysis = build_route_weather_analysis(
        &plan,
        None,
        &[grid(vec![grid_point(
            "dateline",
            10.0,
            180.0,
            WeatherCondition::Cloud,
        )])],
    );
    let longitudes = analysis.layers[0]
        .samples
        .iter()
        .map(|sample| sample.location.longitude.abs())
        .collect::<Vec<_>>();
    assert!(longitudes.iter().all(|longitude| *longitude > 170.0));
}

#[test]
fn matches_forecasts_to_proportional_checkpoint_arrival_times() {
    let departure = Utc.with_ymd_and_hms(2026, 7, 19, 6, 0, 0).unwrap();
    let schedule = schedule(departure, Some(6 * 60 * 60));
    let analysis = build_route_weather_analysis(
        &plan(),
        Some(&schedule),
        &[grid(vec![
            timed_grid_point(
                "departure",
                -34.0,
                151.0,
                WeatherCondition::Clear,
                departure,
            ),
            timed_grid_point(
                "arrival",
                -37.0,
                175.0,
                WeatherCondition::Rain,
                departure + Duration::hours(6),
            ),
        ])],
    );

    assert_eq!(
        analysis.timing.availability,
        RouteWeatherTimingAvailability::Ready
    );
    assert_eq!(analysis.timing.departure_at, Some(departure));
    assert_eq!(analysis.timing.duration_seconds, Some(6 * 60 * 60));
    let samples = &analysis.layers[0].samples;
    assert_eq!(
        samples.first().unwrap().estimated_arrival_at,
        Some(departure)
    );
    assert_eq!(
        samples.last().unwrap().estimated_arrival_at,
        Some(departure + Duration::hours(6))
    );
    assert!(samples.iter().all(|sample| {
        sample.source.as_ref().is_some_and(|source| {
            source.temporal_support == RouteWeatherTemporalSupport::EtaMatched
                && source.time_offset_seconds.is_some()
        })
    }));
}

#[test]
fn uses_schedule_fallbacks_and_keeps_missing_timing_explicit() {
    let departure = Utc.with_ymd_and_hms(2026, 7, 19, 6, 0, 0).unwrap();
    let fallback = PlannedSchedule {
        scheduled_out: Some(departure),
        scheduled_off: None,
        scheduled_on: Some(departure + Duration::hours(5)),
        scheduled_in: Some(departure + Duration::hours(6)),
        estimated_enroute_seconds: Some(8 * 24 * 60 * 60),
    };
    let analysis = build_route_weather_analysis(&plan(), Some(&fallback), &[]);
    assert_eq!(
        analysis.timing.departure_basis,
        Some(RouteWeatherDepartureBasis::ScheduledOut)
    );
    assert_eq!(
        analysis.timing.duration_basis,
        Some(RouteWeatherDurationBasis::ScheduledOn)
    );
    assert_eq!(analysis.timing.duration_seconds, Some(5 * 60 * 60));

    let scheduled_in_fallback = PlannedSchedule {
        scheduled_on: None,
        ..fallback
    };
    let analysis = build_route_weather_analysis(&plan(), Some(&scheduled_in_fallback), &[]);
    assert_eq!(
        analysis.timing.duration_basis,
        Some(RouteWeatherDurationBasis::ScheduledIn)
    );
    assert_eq!(analysis.timing.duration_seconds, Some(6 * 60 * 60));

    let missing = build_route_weather_analysis(&plan(), None, &[]);
    assert_eq!(
        missing.timing.availability,
        RouteWeatherTimingAvailability::DepartureUnavailable
    );
    assert!(
        missing
            .layers
            .iter()
            .flat_map(|layer| &layer.samples)
            .all(|sample| sample.estimated_arrival_at.is_none())
    );
}

#[test]
fn enforces_temporal_support_and_preserves_legacy_current_context() {
    let departure = Utc.with_ymd_and_hms(2026, 7, 19, 6, 0, 0).unwrap();
    let schedule = schedule(departure, Some(60 * 60));
    let inside = departure + Duration::seconds(ROUTE_WEATHER_MAX_TEMPORAL_SUPPORT_SECONDS);
    let outside = inside + Duration::seconds(1);
    let analysis = build_route_weather_analysis(
        &plan(),
        Some(&schedule),
        &[grid(vec![
            timed_grid_point("outside", -34.0, 151.0, WeatherCondition::Rain, outside),
            timed_grid_point("inside", -34.0, 151.0, WeatherCondition::Cloud, inside),
        ])],
    );
    assert_eq!(
        analysis.layers[0].samples[0]
            .source
            .as_ref()
            .unwrap()
            .point_id,
        "inside"
    );

    let unsupported = build_route_weather_analysis(
        &plan(),
        Some(&schedule),
        &[grid(vec![timed_grid_point(
            "outside",
            -34.0,
            151.0,
            WeatherCondition::Rain,
            outside,
        )])],
    );
    assert!(unsupported.layers[0].samples[0].source.is_none());

    let legacy = build_route_weather_analysis(
        &plan(),
        Some(&schedule),
        &[grid(vec![grid_point(
            "legacy",
            -34.0,
            151.0,
            WeatherCondition::Rain,
        )])],
    );
    assert_eq!(legacy.availability, RouteWeatherAvailability::Partial);
    assert!(legacy.layers[0].samples.iter().all(|sample| {
        sample.source.as_ref().is_some_and(|source| {
            source.temporal_support == RouteWeatherTemporalSupport::CurrentContext
                && source.valid_at.is_none()
        })
    }));
}

#[test]
fn exposes_only_the_latest_factual_radar_frame_as_observation_context() {
    let earlier = Utc.with_ymd_and_hms(2026, 7, 19, 5, 50, 0).unwrap();
    let later = earlier + Duration::minutes(10);
    let radar = |frame_time| GlobalWeatherLayerSnapshot {
        schema_version: wyrmgrid_domain::GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "rainviewer-radar".into(),
        title: "Global precipitation RADAR".into(),
        data: GlobalWeatherLayerData::RasterTiles {
            frame_time,
            tiles: Vec::new(),
        },
        provenance: provenance(ProvenanceKind::ExternalFact),
    };
    let analysis = build_route_weather_analysis(&plan(), None, &[radar(later), radar(earlier)]);

    assert_eq!(analysis.radar_contexts.len(), 1);
    assert_eq!(analysis.radar_contexts[0].frame_time, later);
    assert_eq!(
        analysis.radar_contexts[0].relationship,
        RouteWeatherRadarRelationship::ObservationOnly
    );
}

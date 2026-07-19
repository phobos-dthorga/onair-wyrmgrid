use chrono::{TimeZone, Utc};
use wyrmgrid_domain::{
    GlobalWeatherGridPoint, GlobalWeatherLayerData, GlobalWeatherLayerSnapshot,
    OperationalProvenance, ProvenanceKind, SnapshotFreshness, WeatherCondition,
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
        condition,
        temperature_c: Some(12.0),
        precipitation_mm: Some(1.5),
        cloud_cover_percent: Some(80.0),
        wind_direction_degrees: Some(240.0),
        wind_speed_kt: Some(22.0),
        provider_weather_code: Some(61),
    }
}

#[test]
fn samples_resolved_route_segments_and_preserves_source_distance_and_provenance() {
    let analysis = build_route_weather_analysis(
        &plan(),
        &[grid(vec![
            grid_point("west", -34.0, 151.0, WeatherCondition::Rain),
            grid_point("east", -37.0, 175.0, WeatherCondition::Cloud),
        ])],
    );

    assert_eq!(analysis.availability, RouteWeatherAvailability::Ready);
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

    let no_source = build_route_weather_analysis(&plan(), &[]);
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

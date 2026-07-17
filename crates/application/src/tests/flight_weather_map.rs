use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wyrmgrid_domain::{
    AirportWeather, Coordinates, FlightCategory, FlightPlanSnapshot, MetarObservation,
    OperationalObservation, OperationalProvenance, ProvenanceKind, SnapshotFreshness,
    WeatherSnapshot, WeatherSnapshotId,
};

use super::*;

fn plan() -> FlightPlanSnapshot {
    serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/flight-plan-snapshot-v1.json"
    ))
    .unwrap()
}

fn provenance() -> OperationalProvenance {
    OperationalProvenance {
        kind: ProvenanceKind::ExternalFact,
        provider: "aviationweather.gov".into(),
        provider_revision: None,
        generated_at: Some(Utc.with_ymd_and_hms(2026, 7, 16, 1, 0, 0).unwrap()),
        retrieved_at: Utc.with_ymd_and_hms(2026, 7, 16, 1, 1, 0).unwrap(),
        transformation_version: 1,
        freshness: SnapshotFreshness::Current,
    }
}

fn weather() -> WeatherSnapshot {
    WeatherSnapshot {
        schema_version: wyrmgrid_domain::WEATHER_SNAPSHOT_SCHEMA_VERSION,
        id: WeatherSnapshotId(Uuid::parse_str("f6792c25-f2cc-424d-8060-1ed13637f563").unwrap()),
        airports: vec![AirportWeather {
            station_icao: "YSSY".into(),
            metar: Some(OperationalObservation {
                value: MetarObservation {
                    observed_at: Utc.with_ymd_and_hms(2026, 7, 16, 1, 0, 0).unwrap(),
                    raw_text: "METAR YSSY 160100Z 31013KT CAVOK 18/04 Q1021".into(),
                    report_type: Some("METAR".into()),
                    flight_category: Some(FlightCategory::Vfr),
                    wind_direction: None,
                    wind_speed_kt: Some(13),
                    wind_gust_kt: None,
                    visibility_sm: Some("6+".into()),
                    temperature_c: Some(18.0),
                    dewpoint_c: Some(4.0),
                    altimeter_hpa: Some(1021.0),
                    present_weather: None,
                },
                provenance: provenance(),
            }),
            taf: None,
        }],
    }
}

#[test]
fn projection_joins_reports_to_stable_plan_airport_selections() {
    let mut plan = plan();
    plan.airports.value.origin.location = Some(Coordinates {
        latitude: -33.9461,
        longitude: 151.1772,
    });

    let view = build_flight_weather_map_view(&plan, &weather());

    assert_eq!(view.schema_version, FLIGHT_WEATHER_MAP_SCHEMA_VERSION);
    assert_eq!(view.stations[0].id, "weather:origin:yssy");
    assert_eq!(view.stations[0].role, FlightWeatherMapStationRole::Origin);
    assert_eq!(
        view.stations[0].metar.as_ref().unwrap().provenance.provider,
        "aviationweather.gov"
    );
    assert_eq!(
        view.stations.last().unwrap().id,
        "weather:alternate:0000:nzwn"
    );
}

#[test]
fn missing_reports_and_coordinates_remain_explicit_stations() {
    let plan = plan();
    let view = build_flight_weather_map_view(&plan, &weather());
    let destination = view
        .stations
        .iter()
        .find(|station| station.role == FlightWeatherMapStationRole::Destination)
        .unwrap();

    assert_eq!(destination.station_icao, "NZAA");
    assert_eq!(destination.location, None);
    assert_eq!(destination.metar, None);
    assert_eq!(destination.taf, None);
}

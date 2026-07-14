use super::*;
use crate::{OperationalProvenance, ProvenanceKind};

fn provenance(at: DateTime<Utc>) -> OperationalProvenance {
    OperationalProvenance {
        kind: ProvenanceKind::ExternalFact,
        provider: "aviationweather.gov".into(),
        provider_revision: Some("data-api-v4".into()),
        generated_at: Some(at),
        retrieved_at: at,
        transformation_version: 1,
        freshness: SnapshotFreshness::Current,
    }
}

#[test]
fn validates_a_bounded_external_weather_snapshot() {
    let at = DateTime::from_timestamp(1_783_992_600, 0).unwrap();
    let snapshot = WeatherSnapshot {
        schema_version: WEATHER_SNAPSHOT_SCHEMA_VERSION,
        id: WeatherSnapshotId(Uuid::nil()),
        airports: vec![AirportWeather {
            station_icao: "YSSY".into(),
            metar: Some(OperationalObservation {
                value: MetarObservation {
                    observed_at: at,
                    raw_text: "METAR YSSY 140130Z 32007KT CAVOK".into(),
                    report_type: Some("METAR".into()),
                    flight_category: Some(FlightCategory::Vfr),
                    wind_direction: Some(WindDirection::Degrees(320)),
                    wind_speed_kt: Some(7),
                    wind_gust_kt: None,
                    visibility_sm: Some("6+".into()),
                    temperature_c: Some(18.0),
                    dewpoint_c: Some(12.0),
                    altimeter_hpa: Some(1_024.0),
                    present_weather: None,
                },
                provenance: provenance(at),
            }),
            taf: Some(OperationalObservation {
                value: TafForecast {
                    issued_at: at,
                    valid_from: at,
                    valid_to: at + chrono::Duration::hours(24),
                    raw_text: "TAF YSSY 1400/1500 CAVOK".into(),
                },
                provenance: provenance(at),
            }),
        }],
    };

    assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn rejects_duplicate_stations_and_invalid_product_provenance() {
    let at = Utc::now();
    let product = OperationalObservation {
        value: MetarObservation {
            observed_at: at,
            raw_text: "METAR YSSY".into(),
            report_type: None,
            flight_category: None,
            wind_direction: None,
            wind_speed_kt: None,
            wind_gust_kt: None,
            visibility_sm: None,
            temperature_c: None,
            dewpoint_c: None,
            altimeter_hpa: None,
            present_weather: None,
        },
        provenance: OperationalProvenance {
            kind: ProvenanceKind::Calculated,
            ..provenance(at)
        },
    };
    let snapshot = WeatherSnapshot {
        schema_version: WEATHER_SNAPSHOT_SCHEMA_VERSION,
        id: WeatherSnapshotId(Uuid::nil()),
        airports: vec![AirportWeather {
            station_icao: "YSSY".into(),
            metar: Some(product),
            taf: None,
        }],
    };

    assert_eq!(
        snapshot.validate(),
        Err(WeatherValidationError::InvalidProvenance)
    );
}

#[test]
fn validates_the_version_one_json_fixture() {
    let snapshot: WeatherSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/weather-snapshot-v1.json"
    ))
    .unwrap();
    assert_eq!(snapshot.validate(), Ok(()));
}

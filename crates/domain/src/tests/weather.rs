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

#[test]
fn validates_the_global_weather_layer_version_one_fixture() {
    let layer: GlobalWeatherLayerSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/global-weather-layer-v1.json"
    ))
    .unwrap();
    assert_eq!(layer.validate(), Ok(()));
}

#[test]
fn validates_a_provider_neutral_global_grid() {
    let at = DateTime::from_timestamp(1_784_243_200, 0).unwrap();
    let layer = GlobalWeatherLayerSnapshot {
        schema_version: GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "open-meteo-global".into(),
        title: "Global model weather".into(),
        data: GlobalWeatherLayerData::Grid {
            points: vec![GlobalWeatherGridPoint {
                id: "grid-0".into(),
                location: crate::Coordinates {
                    latitude: -33.86,
                    longitude: 151.20,
                },
                valid_at: Some(at),
                condition: WeatherCondition::Rain,
                temperature_c: Some(16.0),
                precipitation_mm: Some(2.5),
                cloud_cover_percent: Some(91.0),
                wind_direction_degrees: Some(180.0),
                wind_speed_kt: Some(17.0),
                provider_weather_code: Some(61),
            }],
        },
        provenance: OperationalProvenance {
            kind: ProvenanceKind::ExternalCalculation,
            provider: "open-meteo.com".into(),
            provider_revision: Some("forecast-api-v1".into()),
            generated_at: Some(at),
            retrieved_at: at,
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        },
    };

    assert_eq!(layer.validate(), Ok(()));
}

#[test]
fn keeps_legacy_global_grid_points_without_a_valid_time_compatible() {
    let layer: GlobalWeatherLayerSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/global-weather-layer-v1.json"
    ))
    .unwrap();
    let GlobalWeatherLayerData::Grid { points } = layer.data else {
        panic!("expected the legacy grid fixture");
    };

    assert_eq!(points[0].valid_at, None);
}

#[test]
fn validates_a_time_aware_global_weather_layer_fixture() {
    let layer: GlobalWeatherLayerSnapshot = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/global-weather-layer-forecast-v1.json"
    ))
    .unwrap();
    assert_eq!(layer.validate(), Ok(()));
    let GlobalWeatherLayerData::Grid { points } = layer.data else {
        panic!("expected a forecast grid fixture");
    };
    assert_eq!(
        points[0].valid_at,
        Some(DateTime::from_timestamp(1_784_293_200, 0).unwrap())
    );
}

#[test]
fn rejects_non_png_and_duplicate_global_raster_tiles() {
    let at = DateTime::from_timestamp(1_784_243_200, 0).unwrap();
    let mut layer = GlobalWeatherLayerSnapshot {
        schema_version: GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "rainviewer-radar".into(),
        title: "Global precipitation radar".into(),
        data: GlobalWeatherLayerData::RasterTiles {
            frame_time: at,
            tiles: vec![GlobalWeatherRasterTile {
                zoom: 1,
                x: 0,
                y: 0,
                png_base64: BASE64_STANDARD.encode(b"not a PNG"),
                coverage_png_base64: None,
            }],
        },
        provenance: OperationalProvenance {
            kind: ProvenanceKind::ExternalFact,
            provider: "rainviewer.com".into(),
            provider_revision: None,
            generated_at: Some(at),
            retrieved_at: at,
            transformation_version: 1,
            freshness: SnapshotFreshness::Current,
        },
    };

    assert_eq!(
        layer.validate(),
        Err(GlobalWeatherValidationError::InvalidRasterTile)
    );

    if let GlobalWeatherLayerData::RasterTiles { tiles, .. } = &mut layer.data {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&256_u32.to_be_bytes());
        png.extend_from_slice(&256_u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        png.extend_from_slice(&[0, 0, 0, 0]);
        png.extend_from_slice(b"\0\0\0\0IEND\0\0\0\0");
        tiles[0].png_base64 = BASE64_STANDARD.encode(png);
        tiles.push(tiles[0].clone());
    }
    assert_eq!(
        layer.validate(),
        Err(GlobalWeatherValidationError::InvalidRasterTile)
    );
}

#[test]
fn validates_optional_radar_coverage_masks_and_counts_their_bytes() {
    let at = DateTime::from_timestamp(1_784_243_200, 0).unwrap();
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&256_u32.to_be_bytes());
    png.extend_from_slice(&256_u32.to_be_bytes());
    png.extend_from_slice(&[8, 6, 0, 0, 0]);
    png.extend_from_slice(&[0, 0, 0, 0]);
    png.extend_from_slice(b"\0\0\0\0IEND\0\0\0\0");
    let mut layer = GlobalWeatherLayerSnapshot {
        schema_version: GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "rainviewer-radar".into(),
        title: "Global precipitation radar".into(),
        data: GlobalWeatherLayerData::RasterTiles {
            frame_time: at,
            tiles: vec![GlobalWeatherRasterTile {
                zoom: 1,
                x: 0,
                y: 0,
                png_base64: BASE64_STANDARD.encode(&png),
                coverage_png_base64: Some(BASE64_STANDARD.encode(&png)),
            }],
        },
        provenance: provenance(at),
    };

    assert_eq!(layer.validate(), Ok(()));
    if let GlobalWeatherLayerData::RasterTiles { tiles, .. } = &mut layer.data {
        tiles[0].coverage_png_base64 = Some(BASE64_STANDARD.encode(b"not a PNG"));
    }
    assert_eq!(
        layer.validate(),
        Err(GlobalWeatherValidationError::InvalidRasterTile)
    );
}

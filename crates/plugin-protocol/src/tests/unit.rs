use super::*;

fn manifest() -> PluginManifest {
    PluginManifest {
        id: "org.wyrmgrid.example.fleet-locations".into(),
        name: "Fleet Locations".into(),
        version: "0.1.0".into(),
        api_version: PLUGIN_API_VERSION,
        author: "Example Developer".into(),
        runtime: Some(PluginRuntime::Python),
        entry_point: "src/main.py".into(),
        permissions: vec![Permission::OnAirFleetRead, Permission::MapLayersPublish],
        weather_capabilities: Vec::new(),
        network_origins: Vec::new(),
    }
}

#[test]
fn accepts_a_safe_manifest() {
    assert_eq!(manifest().validate(), Ok(()));
}

#[test]
fn rejects_unknown_manifest_fields() {
    let manifest = r#"{
        "id":"org.wyrmgrid.example.fleet-locations",
        "name":"Fleet Locations",
        "version":"0.1.0",
        "api_version":1,
        "author":"Example Developer",
        "runtime":"python",
        "entry_point":"src/main.py",
        "unexpected":true
    }"#;

    assert!(serde_json::from_str::<PluginManifest>(manifest).is_err());
}

#[test]
fn general_plugins_cannot_request_audio_capture_or_codec_access() {
    for denied_permission in ["audio_capture", "audio_codec"] {
        let candidate = format!(
            r#"{{
                "id":"org.wyrmgrid.example.audio-request",
                "name":"Denied audio request",
                "version":"0.1.0",
                "api_version":1,
                "author":"Example Developer",
                "runtime":"python",
                "entry_point":"src/main.py",
                "permissions":["{denied_permission}"]
            }}"#,
        );
        assert!(serde_json::from_str::<PluginManifest>(&candidate).is_err());
    }
}

#[test]
fn rejects_parent_directory_entry_points() {
    let mut candidate = manifest();
    candidate.entry_point = "../outside.py".into();
    assert_eq!(candidate.validate(), Err(ManifestError::UnsafeEntryPoint));
}

#[test]
fn rejects_duplicate_manifest_permissions() {
    let mut candidate = manifest();
    candidate.permissions.push(Permission::OnAirFleetRead);
    assert_eq!(
        candidate.validate(),
        Err(ManifestError::DuplicatePermissions)
    );
}

#[test]
fn round_trips_a_bounded_protocol_frame() {
    let message = ProtocolEnvelope::new(
        1,
        HostMessage::Hello {
            host_version: "0.1.0".into(),
            plugin_id: manifest().id,
            granted_permissions: vec![Permission::OnAirFleetRead, Permission::MapLayersPublish],
            weather_capabilities: Vec::new(),
            network_origins: Vec::new(),
        },
    );
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &message).expect("frame should encode");
    let decoded: ProtocolEnvelope<HostMessage> =
        read_frame(&mut bytes.as_slice()).expect("frame should decode");

    assert_eq!(decoded, message);
    assert_eq!(decoded.validate_header(), Ok(()));
}

#[test]
fn validates_weather_manifest_scope_and_bounded_requests() {
    let mut candidate = manifest();
    candidate.permissions = vec![Permission::ExternalNetwork, Permission::WeatherDataPublish];
    candidate.weather_capabilities = vec![WeatherCapability::ForecastGrid];
    candidate.network_origins = vec!["https://api.open-meteo.com".into()];
    assert_eq!(candidate.validate(), Ok(()));

    let request = WeatherRequest {
        id: "open-meteo-1".into(),
        query: WeatherQuery::ForecastGrid {
            points: vec![WeatherGridRequestPoint {
                id: "grid-0".into(),
                location: Coordinates {
                    latitude: -33.86,
                    longitude: 151.20,
                },
            }],
            window: None,
        },
    };
    assert_eq!(request.validate(), Ok(()));
    assert_eq!(request.query.capability(), WeatherCapability::ForecastGrid);
}

#[test]
fn rejects_a_historical_window_even_one_minute_over_the_limit() {
    let starts_at = chrono::DateTime::parse_from_rfc3339("2026-07-12T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let request = WeatherRequest {
        id: "historical-too-wide".into(),
        query: WeatherQuery::ForecastGrid {
            points: vec![WeatherGridRequestPoint {
                id: "grid-0".into(),
                location: Coordinates {
                    latitude: 0.0,
                    longitude: 0.0,
                },
            }],
            window: Some(WeatherTimeWindow {
                target_at: starts_at + chrono::Duration::hours(15),
                starts_at,
                ends_at: starts_at + chrono::Duration::hours(30) + chrono::Duration::minutes(1),
            }),
        },
    };

    assert_eq!(
        request.validate(),
        Err(WeatherRequestError::InvalidTimeWindow)
    );

    let exact_limit = WeatherTimeWindow {
        target_at: starts_at,
        starts_at,
        ends_at: starts_at + chrono::Duration::hours(30),
    };
    assert!(exact_limit.is_valid());

    let target_at_end = WeatherTimeWindow {
        target_at: starts_at + chrono::Duration::hours(30),
        starts_at,
        ends_at: starts_at + chrono::Duration::hours(30),
    };
    assert!(target_at_end.is_valid());
}

#[test]
fn radar_history_offsets_are_bounded_and_omitted_for_legacy_requests() {
    let tile = WeatherTileAddress {
        zoom: 1,
        x: 0,
        y: 0,
    };
    let legacy = WeatherRequest {
        id: "radar-current".into(),
        query: WeatherQuery::RadarTiles {
            tiles: vec![tile],
            frame_offset: None,
        },
    };
    assert_eq!(legacy.validate(), Ok(()));
    let encoded = serde_json::to_value(&legacy).unwrap();
    assert!(encoded["query"].get("frame_offset").is_none());

    let historical = WeatherRequest {
        id: "radar-history".into(),
        query: WeatherQuery::RadarTiles {
            tiles: vec![tile],
            frame_offset: Some(MAX_RADAR_FRAME_OFFSET),
        },
    };
    assert_eq!(historical.validate(), Ok(()));
    assert_eq!(
        serde_json::to_value(&historical).unwrap()["query"]["frame_offset"],
        MAX_RADAR_FRAME_OFFSET
    );

    let excessive = WeatherRequest {
        id: "radar-invalid".into(),
        query: WeatherQuery::RadarTiles {
            tiles: vec![tile],
            frame_offset: Some(MAX_RADAR_FRAME_OFFSET + 1),
        },
    };
    assert_eq!(
        excessive.validate(),
        Err(WeatherRequestError::InvalidRadarFrameOffset)
    );
}

#[test]
fn rejects_weather_scope_widening_without_matching_permissions() {
    let mut candidate = manifest();
    candidate.weather_capabilities = vec![WeatherCapability::RadarTiles];
    assert_eq!(
        candidate.validate(),
        Err(ManifestError::InvalidWeatherCapabilities)
    );

    candidate.weather_capabilities.clear();
    candidate.network_origins = vec!["https://tilecache.rainviewer.com/path".into()];
    candidate.permissions = vec![Permission::ExternalNetwork];
    assert_eq!(
        candidate.validate(),
        Err(ManifestError::InvalidNetworkOrigins)
    );
}

#[test]
fn rejects_an_oversized_frame_before_allocating_it() {
    let mut bytes = ((MAX_FRAME_BYTES as u32) + 1).to_be_bytes().to_vec();
    bytes.extend_from_slice(b"{}");
    assert!(matches!(
        read_frame::<_, serde_json::Value>(&mut bytes.as_slice()),
        Err(FrameError::TooLarge { .. })
    ));
}

#[test]
fn validates_map_layer_coordinates_and_unique_ids() {
    let observed_at = "2026-07-14T00:00:00Z"
        .parse()
        .expect("fixture timestamp should parse");
    let mut layer = MapLayerSpec {
        id: "fleet-locations".into(),
        title: "Fleet locations".into(),
        points: vec![MapPoint {
            id: "VH-WRM".into(),
            label: "VH-WRM".into(),
            location: Coordinates {
                latitude: -33.8688,
                longitude: 151.2093,
            },
        }],
        provenance: Provenance {
            kind: wyrmgrid_domain::ProvenanceKind::Calculated,
            source: "Fleet Locations example plugin".into(),
            observed_at,
        },
    };
    assert_eq!(layer.validate(), Ok(()));

    layer.points.push(layer.points[0].clone());
    assert_eq!(layer.validate(), Err(MapLayerError::InvalidPointId));
}

#[test]
fn validates_the_protocol_version_one_fixtures() {
    let hello: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-host-hello-v1.json"
    ))
    .expect("host fixture should deserialize");
    hello
        .validate_header()
        .expect("host header should validate");

    let ready: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-ready-v1.json"
    ))
    .expect("ready fixture should deserialize");
    ready
        .validate_header()
        .expect("ready header should validate");

    let published: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-map-layer-v1.json"
    ))
    .expect("map-layer fixture should deserialize");
    published
        .validate_header()
        .expect("map-layer header should validate");
    match published.payload {
        PluginMessage::PublishMapLayer { layer } => {
            layer.validate().expect("map layer should validate");
        }
        _ => panic!("fixture should contain a map layer"),
    }

    let telemetry: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-simulator-telemetry-v1.json"
    ))
    .expect("simulator telemetry fixture should deserialize");
    telemetry
        .validate_header()
        .expect("simulator telemetry header should validate");
    match telemetry.payload {
        HostMessage::SimulatorTelemetrySnapshot { snapshot } => snapshot
            .validate()
            .expect("simulator telemetry should validate"),
        _ => panic!("fixture should contain simulator telemetry"),
    }

    let request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-request-v1.json"
    ))
    .expect("weather request fixture should deserialize");
    match request.payload {
        HostMessage::WeatherRequest { request } => {
            request.validate().expect("weather request should validate");
        }
        _ => panic!("fixture should contain a weather request"),
    }

    let historical_request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-historical-request-v1.json"
    ))
    .expect("historical weather request fixture should deserialize");
    match historical_request.payload {
        HostMessage::WeatherRequest { request } => request
            .validate()
            .expect("historical weather request should validate"),
        _ => panic!("fixture should contain a historical weather request"),
    }

    let historical_response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-historical-layer-v1.json"
    ))
    .expect("historical weather response fixture should deserialize");
    match historical_response.payload {
        PluginMessage::PublishWeather {
            response: PluginWeatherResponse::Complete { product },
            ..
        } => assert!(product.validate()),
        _ => panic!("fixture should contain a complete historical weather product"),
    }

    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-layer-v1.json"
    ))
    .expect("weather response fixture should deserialize");
    match response.payload {
        PluginMessage::PublishWeather {
            response: PluginWeatherResponse::Complete { product },
            ..
        } => assert!(product.validate()),
        _ => panic!("fixture should contain a complete weather product"),
    }

    let radar_request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-radar-history-request-v1.json"
    ))
    .expect("RADAR history request fixture should deserialize");
    match radar_request.payload {
        HostMessage::WeatherRequest { request } => {
            request
                .validate()
                .expect("RADAR history request should validate");
        }
        _ => panic!("fixture should contain a RADAR history request"),
    }

    let radar_response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-radar-layer-v1.json"
    ))
    .expect("RADAR response fixture should deserialize");
    match radar_response.payload {
        PluginMessage::PublishWeather {
            response: PluginWeatherResponse::Complete { product },
            ..
        } => assert!(product.validate()),
        _ => panic!("fixture should contain a complete RADAR product"),
    }
}

#[test]
fn validates_the_version_one_chart_fixture() {
    let chart: ChartSpec = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/chart-spec-v1.json"
    ))
    .expect("chart fixture should deserialize");

    assert_eq!(chart.validate(), Ok(()));
    assert_eq!(chart.kind, ChartKind::Area);
    assert_eq!(chart.series.len(), 1);
}

#[test]
fn rejects_non_finite_chart_values() {
    let mut chart: ChartSpec = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/chart-spec-v1.json"
    ))
    .expect("chart fixture should deserialize");
    chart.series[0].points[0].value = f64::NAN;

    assert_eq!(chart.validate(), Err(ChartError::NonFiniteValue));
}

#[test]
fn rejects_oversized_chart_series() {
    let mut chart: ChartSpec = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/chart-spec-v1.json"
    ))
    .expect("chart fixture should deserialize");
    chart.series[0].points = (0..=MAX_CHART_POINTS_PER_SERIES)
        .map(|index| ChartPoint {
            category: index.to_string(),
            value: index as f64,
        })
        .collect();

    assert_eq!(
        chart.validate(),
        Err(ChartError::TooManyPoints {
            maximum: MAX_CHART_POINTS_PER_SERIES,
        })
    );
}

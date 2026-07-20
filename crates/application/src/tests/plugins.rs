use super::*;
use chrono::Utc;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftId, AircraftSummary, CompanyId, CompanySummary, Coordinates, Observed, Provenance,
    ProvenanceKind,
};

fn plugin_view<'a>(status: &'a PluginHostView, plugin_id: &str) -> &'a PluginView {
    status
        .plugins
        .iter()
        .find(|plugin| plugin.id == plugin_id)
        .expect("bundled plugin should be visible")
}

#[test]
fn forecast_provider_receives_only_the_fixed_host_grid() {
    let WeatherQuery::ForecastGrid { points, window } = default_global_weather_grid() else {
        panic!("default global weather query should be a forecast grid");
    };

    assert_eq!(points.len(), 84);
    assert!(window.is_none());
    assert_eq!(
        points.first().map(|point| point.id.as_str()),
        Some("global-0-0")
    );
    assert_eq!(
        points.last().map(|point| point.id.as_str()),
        Some("global-6-11")
    );
    assert!(points.iter().all(|point| point.location.is_valid()));
}

#[test]
fn installs_all_bundled_plugins_with_no_implicit_grants() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );

    let status = service.status().expect("plugin status should load");
    assert!(status.available);
    assert_eq!(status.plugins.len(), 4);
    let bundled = plugin_view(&status, BUNDLED_PLUGIN_ID);
    assert!(bundled.granted_permissions.is_empty());
    assert!(!bundled.start_with_wyrmgrid);
    assert_eq!(bundled.state, PluginProcessState::Stopped);
    for provider_id in [
        OPEN_METEO_PLUGIN_ID,
        AVIATION_WEATHER_PLUGIN_ID,
        RAINVIEWER_PLUGIN_ID,
    ] {
        let provider = plugin_view(&status, provider_id);
        assert!(provider.granted_permissions.is_empty());
        assert!(!provider.weather_capabilities.is_empty());
        assert!(!provider.network_origins.is_empty());
    }
    assert!(matches!(
        service.start(BUNDLED_PLUGIN_ID),
        Err(PluginError::PermissionRequired)
    ));
}

#[test]
fn refreshes_reserved_bundled_plugin_files_when_the_shipped_provider_changes() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    initialize_plugin_root(directory.path()).expect("bundled plugins should install");
    let entry_point = directory
        .path()
        .join(RAINVIEWER_PLUGIN_ID)
        .join("src")
        .join("main.py");
    std::fs::write(&entry_point, "stale bundled provider").expect("stale provider should save");

    initialize_plugin_root(directory.path()).expect("bundled plugins should refresh");

    assert_eq!(
        std::fs::read_to_string(entry_point).expect("provider should read"),
        RAINVIEWER_ENTRY_POINT
    );
}

#[test]
fn host_owned_weather_configuration_is_bounded_and_persistent() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store.clone()),
        SimulatorBridgeService::new(Vec::new()),
    );

    let status = service.status().expect("plugin status should load");
    let forecast = &plugin_view(&status, OPEN_METEO_PLUGIN_ID).configuration[0];
    assert_eq!(forecast.key, FORECAST_REFRESH_SETTING_KEY);
    assert_eq!(forecast.value, "15");
    assert_eq!(forecast.choices.len(), 4);
    let radar = &plugin_view(&status, RAINVIEWER_PLUGIN_ID).configuration[0];
    assert_eq!(radar.key, RADAR_REFRESH_SETTING_KEY);
    assert_eq!(radar.value, "5");
    assert!(
        plugin_view(&status, BUNDLED_PLUGIN_ID)
            .configuration
            .is_empty()
    );

    let revised = service
        .set_configuration(OPEN_METEO_PLUGIN_ID, FORECAST_REFRESH_SETTING_KEY, "60")
        .expect("host setting should save");
    assert_eq!(
        plugin_view(&revised, OPEN_METEO_PLUGIN_ID).configuration[0].value,
        "60"
    );
    let manifest = service
        .find_plugin(OPEN_METEO_PLUGIN_ID)
        .expect("bundled weather plugin should exist")
        .manifest;
    assert_eq!(
        service
            .weather_refresh_intervals(&manifest)
            .unwrap()
            .get(&WeatherCapability::ForecastGrid),
        Some(&Duration::from_secs(60 * 60))
    );
    assert!(matches!(
        service.set_configuration(OPEN_METEO_PLUGIN_ID, FORECAST_REFRESH_SETTING_KEY, "7"),
        Err(PluginError::InvalidConfiguration)
    ));
    assert!(matches!(
        service.set_configuration(OPEN_METEO_PLUGIN_ID, "provider_api_key", "secret"),
        Err(PluginError::UnknownConfiguration)
    ));

    let reopened = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    assert_eq!(
        plugin_view(&reopened.status().unwrap(), OPEN_METEO_PLUGIN_ID).configuration[0].value,
        "60"
    );
}

fn radar_layer(frame_time: chrono::DateTime<Utc>, title: &str) -> GlobalWeatherLayerSnapshot {
    GlobalWeatherLayerSnapshot {
        schema_version: wyrmgrid_domain::GLOBAL_WEATHER_LAYER_SCHEMA_VERSION,
        id: "rainviewer-radar".into(),
        title: title.into(),
        time_scope: None,
        data: wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles {
            frame_time,
            tiles: vec![wyrmgrid_domain::GlobalWeatherRasterTile {
                zoom: 1,
                x: 0,
                y: 0,
                png_base64: "unused-by-history-test".into(),
                coverage_png_base64: None,
            }],
        },
        provenance: wyrmgrid_domain::OperationalProvenance {
            kind: ProvenanceKind::ExternalFact,
            provider: "rainviewer.com".into(),
            provider_revision: Some("weather-maps-v2".into()),
            generated_at: Some(frame_time),
            retrieved_at: frame_time,
            transformation_version: 1,
            freshness: wyrmgrid_domain::SnapshotFreshness::Current,
        },
    }
}

#[test]
fn radar_history_is_ordered_deduplicated_and_bounded_in_memory() {
    let mut layers = BTreeMap::new();
    let base = chrono::DateTime::from_timestamp(1_784_290_000, 0).unwrap();
    for offset in [3_i64, 1, 2] {
        insert_weather_layer(
            &mut layers,
            radar_layer(base + chrono::Duration::minutes(offset * 10), "RADAR"),
        );
    }
    insert_weather_layer(
        &mut layers,
        radar_layer(base + chrono::Duration::minutes(20), "Updated RADAR"),
    );

    let history = &layers["rainviewer-radar"];
    assert_eq!(history.len(), 3);
    assert_eq!(history[1].title, "Updated RADAR");
    for offset in [4_i64, 5, 6, 7] {
        insert_weather_layer(
            &mut layers,
            radar_layer(base + chrono::Duration::minutes(offset * 10), "RADAR"),
        );
    }
    let history = &layers["rainviewer-radar"];
    assert_eq!(history.len(), MAX_RADAR_FRAMES_PER_LAYER);
    let times = history
        .iter()
        .map(|layer| match layer.data {
            wyrmgrid_domain::GlobalWeatherLayerData::RasterTiles { frame_time, .. } => frame_time,
            wyrmgrid_domain::GlobalWeatherLayerData::Grid { .. } => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert!(times.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(times[0], base + chrono::Duration::minutes(20));
}

#[test]
fn automatic_start_requires_and_remembers_current_standing_access() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store.clone()),
        SimulatorBridgeService::new(Vec::new()),
    );

    service
        .approve_requested_permissions_with_lifetime(
            BUNDLED_PLUGIN_ID,
            AuthorizationGrantLifetime::Session,
        )
        .expect("session permission should approve");
    assert!(matches!(
        service.set_start_with_wyrmgrid(BUNDLED_PLUGIN_ID, true),
        Err(PluginError::StandingPermissionRequired)
    ));
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("standing permissions should approve");
    let enabled = service
        .set_start_with_wyrmgrid(BUNDLED_PLUGIN_ID, true)
        .expect("automatic startup should enable");
    assert!(plugin_view(&enabled, BUNDLED_PLUGIN_ID).start_with_wyrmgrid);

    let reopened = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    assert!(plugin_view(&reopened.status().unwrap(), BUNDLED_PLUGIN_ID).start_with_wyrmgrid);

    let revoked = reopened
        .revoke_permissions(BUNDLED_PLUGIN_ID)
        .expect("access should revoke");
    assert!(!plugin_view(&revoked, BUNDLED_PLUGIN_ID).start_with_wyrmgrid);
}

#[test]
fn plugin_scope_changes_disable_automatic_start_until_reviewed_again() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");
    service
        .set_start_with_wyrmgrid(BUNDLED_PLUGIN_ID, true)
        .expect("automatic startup should enable");

    let manifest_path = directory.path().join(BUNDLED_PLUGIN_ID).join("plugin.json");
    let mut manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path).expect("manifest should read"),
    )
    .expect("manifest should parse");
    manifest["version"] = serde_json::Value::String("0.2.0".into());
    std::fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should save");

    let status = service.status().expect("status should load");
    assert!(!plugin_view(&status, BUNDLED_PLUGIN_ID).start_with_wyrmgrid);
}

#[test]
fn persists_and_revokes_only_the_requested_capabilities() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );

    let approved = service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");
    assert_eq!(
        plugin_view(&approved, BUNDLED_PLUGIN_ID)
            .granted_permissions
            .len(),
        2
    );

    let revoked = service
        .revoke_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should revoke");
    assert!(
        plugin_view(&revoked, BUNDLED_PLUGIN_ID)
            .granted_permissions
            .is_empty()
    );
}

#[test]
fn plugin_version_changes_require_a_fresh_permission_review() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("initial permissions should approve");

    let manifest_path = directory.path().join(BUNDLED_PLUGIN_ID).join("plugin.json");
    let mut manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path).expect("manifest should read"),
    )
    .expect("manifest should parse");
    manifest["version"] = serde_json::Value::String("0.2.0".into());
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("updated manifest should save");

    let status = service
        .status()
        .expect("updated plugin should remain visible");
    assert!(
        plugin_view(&status, BUNDLED_PLUGIN_ID)
            .granted_permissions
            .is_empty()
    );
    assert!(matches!(
        service.start(BUNDLED_PLUGIN_ID),
        Err(PluginError::PermissionRequired)
    ));
}

#[test]
fn completes_the_out_of_process_handshake_when_python_is_available() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");

    let started = match service.start(BUNDLED_PLUGIN_ID) {
        Ok(status) => status,
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("plugin should start: {error}"),
    };
    assert_eq!(
        plugin_view(&started, BUNDLED_PLUGIN_ID).state,
        PluginProcessState::Running
    );

    let stopped = service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
    assert_eq!(
        plugin_view(&stopped, BUNDLED_PLUGIN_ID).state,
        PluginProcessState::Stopped
    );
}

#[test]
fn one_launch_permission_is_not_shown_as_active_after_the_plugin_stops() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions_with_lifetime(
            BUNDLED_PLUGIN_ID,
            AuthorizationGrantLifetime::Once,
        )
        .expect("one launch should approve");

    let started = match service.start(BUNDLED_PLUGIN_ID) {
        Ok(status) => status,
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("plugin should start once: {error}"),
    };
    assert_eq!(
        plugin_view(&started, BUNDLED_PLUGIN_ID).grant_lifetime,
        Some(AuthorizationGrantLifetime::Once)
    );

    let stopped = service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
    let bundled = plugin_view(&stopped, BUNDLED_PLUGIN_ID);
    assert!(bundled.granted_permissions.is_empty());
    assert_eq!(bundled.grant_lifetime, None);
    assert!(matches!(
        service.start(BUNDLED_PLUGIN_ID),
        Err(PluginError::PermissionRequired)
    ));
}

#[test]
fn manual_stop_preserves_the_saved_automatic_start_choice() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");
    service
        .set_start_with_wyrmgrid(BUNDLED_PLUGIN_ID, true)
        .expect("automatic startup should enable");
    match service.start(BUNDLED_PLUGIN_ID) {
        Ok(_) => {}
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("plugin should start: {error}"),
    }

    let stopped = service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
    let plugin = plugin_view(&stopped, BUNDLED_PLUGIN_ID);
    assert_eq!(plugin.state, PluginProcessState::Stopped);
    assert!(plugin.start_with_wyrmgrid);
}

#[test]
fn automatic_start_isolates_a_failed_plugin_from_the_others() {
    const FAILED_PLUGIN_ID: &str = "org.wyrmgrid.test.failed-startup";
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let failed_root = directory.path().join(FAILED_PLUGIN_ID);
    let failed_source = failed_root.join("src");
    std::fs::create_dir_all(&failed_source).expect("test plugin directory should create");
    std::fs::write(
        failed_root.join("plugin.json"),
        r#"{
          "id":"org.wyrmgrid.test.failed-startup",
          "name":"Failed Startup Test",
          "version":"0.1.0",
          "api_version":1,
          "author":"WyrmGrid tests",
          "runtime":"python",
          "entry_point":"src/main.py",
          "permissions":["map_layers_publish"]
        }"#,
    )
    .expect("test manifest should write");
    std::fs::write(failed_source.join("main.py"), "raise SystemExit(1)\n")
        .expect("test entry point should write");

    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    for plugin_id in [BUNDLED_PLUGIN_ID, FAILED_PLUGIN_ID] {
        service
            .approve_requested_permissions(plugin_id)
            .expect("standing permissions should approve");
        service
            .set_start_with_wyrmgrid(plugin_id, true)
            .expect("automatic startup should enable");
    }

    let outcome = service
        .start_enabled()
        .expect("individual startup failures should be contained");
    if outcome
        .started_plugin_ids
        .contains(&BUNDLED_PLUGIN_ID.to_owned())
    {
        assert_eq!(outcome.started_plugin_ids, vec![BUNDLED_PLUGIN_ID]);
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].plugin_id, FAILED_PLUGIN_ID);
        let status = service.status().expect("status should remain available");
        assert!(plugin_view(&status, FAILED_PLUGIN_ID).last_error.is_some());
        service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
    } else {
        assert!(outcome.started_plugin_ids.is_empty());
        assert_eq!(outcome.failures.len(), 2);
        assert!(
            outcome
                .failures
                .iter()
                .all(|failure| failure.message == PluginError::RuntimeUnavailable.to_string())
        );
    }
}

#[test]
fn publishes_a_host_validated_layer_from_a_sanitized_fleet_snapshot() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let mut store = Store::open_in_memory().expect("store should open");
    let company_id = CompanyId(
        Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").expect("company id should parse"),
    );
    let observed_at = Utc::now();
    let stored = crate::StoredFleetSnapshot {
        schema_version: crate::FLEET_SNAPSHOT_SCHEMA_VERSION,
        company: CompanySummary {
            id: company_id.clone(),
            name: "Sanitized Test Company".into(),
            airline_code: "WYR".into(),
        },
        snapshot: Observed {
            value: vec![AircraftSummary {
                id: AircraftId(
                    Uuid::parse_str("11111111-1111-4111-8111-111111111111")
                        .expect("aircraft id should parse"),
                ),
                registration: Some("WYR-101".into()),
                model: Some("Example Turboprop".into()),
                location: Some(Coordinates {
                    latitude: -37.8136,
                    longitude: 144.9631,
                }),
                current_airport: None,
            }],
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: "sanitized-test-fixture".into(),
                observed_at,
            },
        },
    };
    store
        .save_api_snapshot(
            crate::FLEET_RESOURCE_KIND,
            &company_id.0.to_string(),
            &observed_at.to_rfc3339(),
            &serde_json::to_string(&stored).expect("snapshot should serialize"),
        )
        .expect("snapshot should save");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");
    match service.start(BUNDLED_PLUGIN_ID) {
        Ok(_) => {}
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("plugin should start: {error}"),
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    let published = loop {
        let status = service.status().expect("status should load");
        if let Some(layer) = status.layers.first() {
            break layer.clone();
        }
        assert!(Instant::now() < deadline, "plugin should publish a layer");
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(published.layer.id, "fleet-locations");
    assert_eq!(published.layer.points.len(), 1);
    assert_eq!(
        published.layer.points[0].label,
        "WYR-101 · Example Turboprop"
    );
    assert_eq!(published.layer.provenance.kind, ProvenanceKind::Calculated);
    service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
}

#[test]
fn weather_products_must_match_the_exact_host_request() {
    let request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-request-v1.json"
    ))
    .expect("request fixture should parse");
    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-layer-v1.json"
    ))
    .expect("response fixture should parse");
    let HostMessage::WeatherRequest { request } = request.payload else {
        panic!("fixture should contain a request");
    };
    let PluginMessage::PublishWeather { mut response, .. } = response.payload else {
        panic!("fixture should contain a response");
    };
    assert!(weather_response_matches_request(&response, &request.query));

    let PluginWeatherResponse::Complete { product } = &mut response else {
        panic!("fixture should contain a global layer");
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        panic!("fixture should contain a global layer");
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        panic!("fixture should contain grid points");
    };
    points[0].location.longitude = 0.0;
    assert!(!weather_response_matches_request(&response, &request.query));
}

#[test]
fn historical_weather_products_require_the_exact_window_and_classification() {
    let request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-historical-request-v1.json"
    ))
    .expect("historical request fixture should parse");
    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-historical-layer-v1.json"
    ))
    .expect("historical response fixture should parse");
    let HostMessage::WeatherRequest { request } = request.payload else {
        panic!("fixture should contain a request");
    };
    let PluginMessage::PublishWeather { response, .. } = response.payload else {
        panic!("fixture should contain a response");
    };
    assert!(weather_response_matches_request(&response, &request.query));

    let mut unclassified = response.clone();
    let PluginWeatherResponse::Complete { product } = &mut unclassified else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    layer.time_scope = None;
    assert!(!weather_response_matches_request(
        &unclassified,
        &request.query
    ));

    let mut out_of_window = response;
    let PluginWeatherResponse::Complete { product } = &mut out_of_window else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        unreachable!();
    };
    points[0].valid_at = Some(Utc::now());
    assert!(!weather_response_matches_request(
        &out_of_window,
        &request.query
    ));
}

#[test]
fn forecast_products_may_return_bounded_timed_horizons_for_each_requested_point() {
    let request: ProtocolEnvelope<HostMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-request-v1.json"
    ))
    .expect("request fixture should parse");
    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-layer-v1.json"
    ))
    .expect("response fixture should parse");
    let HostMessage::WeatherRequest { request } = request.payload else {
        panic!("fixture should contain a request");
    };
    let PluginMessage::PublishWeather { mut response, .. } = response.payload else {
        panic!("fixture should contain a response");
    };
    let PluginWeatherResponse::Complete { product } = &mut response else {
        panic!("fixture should contain a global layer");
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        panic!("fixture should contain a global layer");
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        panic!("fixture should contain grid points");
    };
    let requested_point = points[0].clone();
    *points = [0, 3, 6, 9, 12, 18]
        .into_iter()
        .map(|horizon| {
            let mut point = requested_point.clone();
            point.id = format!("{}-h{horizon:02}", requested_point.id);
            point.valid_at = Some(Utc::now() + chrono::TimeDelta::hours(horizon));
            point
        })
        .collect();

    assert!(weather_response_matches_request(&response, &request.query));

    let mut missing_time = response.clone();
    let PluginWeatherResponse::Complete { product } = &mut missing_time else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        unreachable!();
    };
    points[0].valid_at = None;
    assert!(!weather_response_matches_request(
        &missing_time,
        &request.query
    ));

    let mut malformed_horizon = response.clone();
    let PluginWeatherResponse::Complete { product } = &mut malformed_horizon else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        unreachable!();
    };
    points[0].id = format!("{}-h3", requested_point.id);
    assert!(!weather_response_matches_request(
        &malformed_horizon,
        &request.query
    ));
}

#[test]
fn bundled_open_meteo_shape_correlates_all_504_forecast_points() {
    let query = default_global_weather_grid();
    let WeatherQuery::ForecastGrid {
        points: requested_points,
        window: _,
    } = &query
    else {
        unreachable!();
    };
    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-layer-v1.json"
    ))
    .expect("response fixture should parse");
    let PluginMessage::PublishWeather { mut response, .. } = response.payload else {
        panic!("fixture should contain a response");
    };
    let PluginWeatherResponse::Complete { product } = &mut response else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        unreachable!();
    };
    let template = points[0].clone();
    *points = requested_points
        .iter()
        .flat_map(|requested| {
            [0, 3, 6, 9, 12, 18].into_iter().map(|horizon| {
                let mut point = template.clone();
                point.id = format!("{}-h{horizon:02}", requested.id);
                point.location = requested.location;
                point.valid_at = Some(Utc::now() + chrono::TimeDelta::hours(horizon));
                point
            })
        })
        .collect();

    assert_eq!(points.len(), 504);
    assert!(weather_response_matches_request(&response, &query));
}

#[test]
fn forecast_horizon_correlation_rejects_ambiguous_requested_prefixes() {
    let location = Coordinates {
        latitude: -37.8136,
        longitude: 144.9631,
    };
    let query = WeatherQuery::ForecastGrid {
        points: vec![
            WeatherGridRequestPoint {
                id: "grid".into(),
                location,
            },
            WeatherGridRequestPoint {
                id: "grid-h00".into(),
                location,
            },
        ],
        window: None,
    };
    let response: ProtocolEnvelope<PluginMessage> = serde_json::from_str(include_str!(
        "../../../../schemas/fixtures/plugin-weather-layer-v1.json"
    ))
    .expect("response fixture should parse");
    let PluginMessage::PublishWeather { mut response, .. } = response.payload else {
        panic!("fixture should contain a response");
    };
    let PluginWeatherResponse::Complete { product } = &mut response else {
        unreachable!();
    };
    let PluginWeatherProduct::GlobalLayer { layer } = product else {
        unreachable!();
    };
    let wyrmgrid_domain::GlobalWeatherLayerData::Grid { points } = &mut layer.data else {
        unreachable!();
    };
    points[0].id = "grid-h00".into();
    points[0].location = location;
    points[0].valid_at = Some(Utc::now());

    assert!(!weather_response_matches_request(&response, &query));
}

#[test]
fn plugin_runtime_failures_emit_stable_bounded_diagnostics() {
    let diagnostic = PluginRuntimeFailure::WeatherProductRequestMismatch
        .diagnostic("org.wyrmgrid.provider.open-meteo");

    assert_eq!(diagnostic.level, "error");
    assert_eq!(diagnostic.code, "plugin.weather_product_request_mismatch");
    assert_eq!(diagnostic.operation, "plugin_weather");
    assert_eq!(
        diagnostic.message,
        "The plugin weather product did not match the bounded host request."
    );
    assert_eq!(diagnostic.plugin_id, "org.wyrmgrid.provider.open-meteo");
    assert!(diagnostic.reportable);
    assert!(!diagnostic.message.contains(&diagnostic.plugin_id));
}

#[test]
fn plugin_runtime_preserves_only_the_first_failure_transition() {
    let state = Mutex::new(PluginProcessState::Running);
    let failure_recorded = AtomicBool::new(false);

    assert!(mark_plugin_failed(&state, &failure_recorded));
    assert!(!mark_plugin_failed(&state, &failure_recorded));
    assert_eq!(
        *state.lock().expect("state should remain available"),
        PluginProcessState::Failed
    );
}

#[test]
fn provider_origin_or_product_changes_require_fresh_review() {
    let mut manifest: PluginManifest =
        serde_json::from_str(OPEN_METEO_MANIFEST).expect("provider manifest should parse");
    let original = plugin_scope_revision(&manifest);
    manifest.network_origins = vec!["https://customer-api.open-meteo.com".into()];
    assert_ne!(plugin_scope_revision(&manifest), original);
    manifest.network_origins = vec!["https://api.open-meteo.com".into()];
    manifest.weather_capabilities = vec![WeatherCapability::RadarTiles];
    assert_ne!(plugin_scope_revision(&manifest), original);
}

#[test]
fn dispatch_can_request_airport_weather_from_a_supervised_plugin() {
    const PLUGIN_ID: &str = "org.wyrmgrid.test.airport-weather";
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    let plugin_root = directory.path().join(PLUGIN_ID);
    let source_root = plugin_root.join("src");
    let sdk_root = source_root.join("wyrmgrid_sdk");
    std::fs::create_dir_all(&sdk_root).expect("test plugin directory should create");
    std::fs::write(
        plugin_root.join("plugin.json"),
        r#"{
          "id":"org.wyrmgrid.test.airport-weather",
          "name":"Test Airport Weather",
          "version":"0.1.0",
          "api_version":1,
          "author":"WyrmGrid tests",
          "runtime":"python",
          "entry_point":"src/main.py",
          "permissions":["external_network","weather_data_publish"],
          "weather_capabilities":["airport_reports"],
          "network_origins":["https://example.test"]
        }"#,
    )
    .expect("test manifest should write");
    std::fs::write(sdk_root.join("__init__.py"), BUNDLED_PYTHON_SDK)
        .expect("test SDK should write");
    std::fs::write(
        source_root.join("main.py"),
        r#"from wyrmgrid_sdk import Plugin

def reports(request, _http):
    stations = request["query"]["stations"]
    return {
        "kind": "airport_reports",
        "snapshot": {
            "schema_version": 1,
            "id": "00000000-0000-0000-0000-000000000000",
            "airports": [{"station_icao": station} for station in stations],
        },
    }

Plugin(
    plugin_id="org.wyrmgrid.test.airport-weather",
    on_weather_request=reports,
).run()
"#,
    )
    .expect("test plugin should write");

    service
        .approve_requested_permissions(PLUGIN_ID)
        .expect("test provider permissions should approve");
    match service.start(PLUGIN_ID) {
        Ok(_) => {}
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("test provider should start: {error}"),
    }
    let snapshot = service
        .request_airport_weather(&["YSSY".into(), "NZAA".into()], None)
        .expect("airport weather should return");
    assert_eq!(snapshot.airports.len(), 2);
    assert_eq!(snapshot.airports[0].station_icao, "YSSY");
    assert_eq!(snapshot.airports[1].station_icao, "NZAA");
    service.stop(PLUGIN_ID).expect("test provider should stop");
}

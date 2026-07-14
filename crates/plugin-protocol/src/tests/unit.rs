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

use super::*;
use chrono::Utc;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftId, AircraftSummary, CompanyId, CompanySummary, Coordinates, Observed, Provenance,
    ProvenanceKind,
};

#[test]
fn installs_the_bundled_plugin_with_no_implicit_grants() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
    );

    let status = service.status().expect("plugin status should load");
    assert!(status.available);
    assert_eq!(status.plugins.len(), 1);
    assert_eq!(status.plugins[0].id, BUNDLED_PLUGIN_ID);
    assert!(status.plugins[0].granted_permissions.is_empty());
    assert_eq!(status.plugins[0].state, PluginProcessState::Stopped);
}

#[test]
fn persists_and_revokes_only_the_requested_capabilities() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
    );

    let approved = service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");
    assert_eq!(approved.plugins[0].granted_permissions.len(), 2);

    let revoked = service
        .revoke_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should revoke");
    assert!(revoked.plugins[0].granted_permissions.is_empty());
}

#[test]
fn completes_the_out_of_process_handshake_when_python_is_available() {
    let directory = tempfile::tempdir().expect("temporary directory should open");
    let store = Store::open_in_memory().expect("store should open");
    let service = PluginService::new(
        Some(directory.path().to_path_buf()),
        store.clone(),
        OnAirSession::with_default_store(store),
    );
    service
        .approve_requested_permissions(BUNDLED_PLUGIN_ID)
        .expect("permissions should approve");

    let started = match service.start(BUNDLED_PLUGIN_ID) {
        Ok(status) => status,
        Err(PluginError::RuntimeUnavailable) => return,
        Err(error) => panic!("plugin should start: {error}"),
    };
    assert_eq!(started.plugins[0].state, PluginProcessState::Running);

    let stopped = service.stop(BUNDLED_PLUGIN_ID).expect("plugin should stop");
    assert_eq!(stopped.plugins[0].state, PluginProcessState::Stopped);
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

use super::*;
use chrono::{Duration, SecondsFormat, Timelike, Utc};

#[test]
fn initializes_the_database_schema() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert_eq!(
        store.schema_version().expect("version should be readable"),
        20
    );
}

#[test]
fn extension_package_migration_is_idempotent_after_version_nineteen() {
    let connection = Connection::open_in_memory().expect("database should open");
    connection.execute_batch(INITIAL_SCHEMA).unwrap();
    connection
        .execute_batch(FLIGHT_OPERATION_AIRCRAFT_ASSIGNMENTS_SCHEMA)
        .unwrap();
    connection.execute_batch(EXTENSION_PACKAGES_SCHEMA).unwrap();
    connection.execute_batch(EXTENSION_PACKAGES_SCHEMA).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        20
    );
    let tables: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN (
                'extension_package_versions', 'extension_package_state',
                'audio_provider_preferences'
             )",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 3);
}

#[test]
fn extension_package_versions_activate_disable_and_rollback_without_overwrite() {
    let store = Store::open_in_memory().expect("store should initialize");
    let version_one = new_extension_package("1.0.0", 'a');
    let first_state = store
        .activate_extension_package_version_record(&version_one)
        .unwrap();
    assert_eq!(first_state.active_version, "1.0.0");
    assert_eq!(first_state.rollback_version, None);
    assert!(first_state.enabled);

    store
        .set_extension_package_enabled("ordinary_plugin", &version_one.extension_id, false)
        .unwrap();
    let version_two = new_extension_package("1.1.0", 'b');
    let updated_state = store
        .activate_extension_package_version_record(&version_two)
        .unwrap();
    assert_eq!(updated_state.active_version, "1.1.0");
    assert_eq!(updated_state.rollback_version.as_deref(), Some("1.0.0"));
    assert!(!updated_state.enabled);

    let rolled_back = store
        .rollback_extension_package("ordinary_plugin", &version_one.extension_id)
        .unwrap();
    assert_eq!(rolled_back.active_version, "1.0.0");
    assert_eq!(rolled_back.rollback_version.as_deref(), Some("1.1.0"));
    assert!(!rolled_back.enabled);
    assert_eq!(
        store
            .list_extension_package_version_records("ordinary_plugin", &version_one.extension_id,)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn extension_package_version_identity_rejects_different_archive_content() {
    let store = Store::open_in_memory().expect("store should initialize");
    let version = new_extension_package("1.0.0", 'a');
    store
        .activate_extension_package_version_record(&version)
        .unwrap();
    let conflicting = NewExtensionPackageVersionRecord {
        archive_sha256: "b".repeat(64),
        ..version
    };
    assert!(matches!(
        store.activate_extension_package_version_record(&conflicting),
        Err(StorageError::InvalidRecord)
    ));
}

#[test]
fn first_party_seed_adds_a_version_without_downgrading_an_existing_active_version() {
    let store = Store::open_in_memory().expect("store should initialize");
    let bundled = NewExtensionPackageVersionRecord {
        source: "first_party".into(),
        ..new_extension_package("1.0.0", 'a')
    };
    store
        .seed_extension_package_version_record(&bundled)
        .unwrap();
    let update = new_extension_package("2.0.0", 'b');
    store
        .activate_extension_package_version_record(&update)
        .unwrap();

    let seeded_again = store
        .seed_extension_package_version_record(&bundled)
        .unwrap();
    assert_eq!(seeded_again.active_version, "2.0.0");
    assert_eq!(seeded_again.rollback_version.as_deref(), Some("1.0.0"));
}

#[test]
fn persists_simulator_provider_package_state_independently_from_plugins() {
    let store = Store::open_in_memory().expect("store should initialize");
    let plugin = new_extension_package("1.0.0", 'a');
    store
        .activate_extension_package_version_record(&plugin)
        .unwrap();
    let provider = NewExtensionPackageVersionRecord {
        package_kind: "simulator_provider".into(),
        extension_id: "org.wyrmgrid.test.simulator-provider".into(),
        archive_sha256: "b".repeat(64),
        extension_manifest_json: "{\"kind\":\"provider\"}".into(),
        ..plugin.clone()
    };
    store
        .activate_extension_package_version_record(&provider)
        .unwrap();

    assert_eq!(
        store
            .list_extension_package_state_records("ordinary_plugin")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .list_extension_package_state_records("simulator_provider")
            .unwrap()[0]
            .extension_id,
        provider.extension_id
    );
}

#[test]
fn persists_audio_provider_package_state_and_selection_independently() {
    let store = Store::open_in_memory().expect("store should initialize");
    let audio = NewExtensionPackageVersionRecord {
        package_kind: "audio_provider".into(),
        extension_id: "org.wyrmgrid.test.audio-provider".into(),
        extension_manifest_json: "{\"kind\":\"audio_provider\"}".into(),
        ..new_extension_package("1.0.0", 'c')
    };
    store
        .activate_extension_package_version_record(&audio)
        .unwrap();
    store
        .save_audio_provider_preferences_record(&AudioProviderPreferencesRecord {
            selected_provider_id: Some(audio.extension_id.clone()),
        })
        .unwrap();

    assert_eq!(
        store
            .list_extension_package_state_records("audio_provider")
            .unwrap()[0]
            .extension_id,
        audio.extension_id
    );
    assert_eq!(
        store
            .load_audio_provider_preferences_record()
            .unwrap()
            .unwrap()
            .selected_provider_id,
        Some(audio.extension_id)
    );
}

#[test]
fn extension_package_removal_deletes_state_and_version_history() {
    let store = Store::open_in_memory().expect("store should initialize");
    let version = new_extension_package("1.0.0", 'a');
    store
        .activate_extension_package_version_record(&version)
        .unwrap();
    store
        .delete_extension_package_records("ordinary_plugin", &version.extension_id)
        .unwrap();
    assert_eq!(
        store
            .load_extension_package_state_record("ordinary_plugin", &version.extension_id)
            .unwrap(),
        None
    );
    assert!(
        store
            .list_extension_package_version_records("ordinary_plugin", &version.extension_id,)
            .unwrap()
            .is_empty()
    );
}

fn new_extension_package(
    version: &str,
    digest_character: char,
) -> NewExtensionPackageVersionRecord {
    NewExtensionPackageVersionRecord {
        package_kind: "ordinary_plugin".into(),
        extension_id: "org.wyrmgrid.test.package".into(),
        version: version.into(),
        archive_sha256: digest_character.to_string().repeat(64),
        package_schema_version: 1,
        source: "local_file".into(),
        package_manifest_json: "{}".into(),
        extension_manifest_json: "{}".into(),
    }
}

#[test]
fn audio_consent_defaults_off_and_round_trips_source_specific_choices() {
    let store = Store::open_in_memory().expect("store should initialize");
    assert_eq!(
        store.load_audio_recording_preferences_record().unwrap(),
        None
    );

    let preferences = AudioRecordingPreferencesRecord {
        enabled: true,
        capture_manual: true,
        capture_automatic: false,
        retention_days: 14,
        storage_budget_bytes: 64 * 1024 * 1024,
    };
    store
        .save_audio_recording_preferences_record(&preferences)
        .unwrap();
    assert_eq!(
        store.load_audio_recording_preferences_record().unwrap(),
        Some(preferences)
    );

    let selection = AudioSourceSelectionRecord {
        provider_id: "dev.wyrmgrid.fake-audio".into(),
        source_id: "synthetic.microphone.primary".into(),
        profile_id: "pilot_microphone_v1".into(),
        enabled: true,
        playback_muted: false,
        playback_solo: false,
        playback_volume_percent: 85,
    };
    store
        .save_audio_source_selection_record(&selection)
        .unwrap();
    assert_eq!(
        store.list_audio_source_selection_records().unwrap(),
        vec![selection]
    );
}

#[test]
fn malformed_persisted_audio_consent_fails_closed() {
    let store = Store::open_in_memory().expect("store should initialize");
    {
        let connection = store.connection.lock().unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        connection.execute(
            "INSERT INTO audio_recording_preferences
                (singleton_id, enabled, capture_manual, capture_automatic, retention_days, storage_budget_bytes)
             VALUES (1, 2, 0, 0, 30, 5368709120)",
            [],
        ).unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints = OFF;")
            .unwrap();
    }
    assert!(matches!(
        store.load_audio_recording_preferences_record(),
        Err(StorageError::InvalidRecord)
    ));
}

#[test]
fn pinned_linked_sessions_are_excluded_from_audio_retention_candidates() {
    let store = Store::open_in_memory().expect("store should initialize");
    store
        .create_simulator_session_record(&SimulatorSessionRecord {
            id: "sim-pinned".into(),
            provider_id: "dev.wyrmgrid.simulator".into(),
            simulator_family: "Test".into(),
            simulator_version: None,
            aircraft_title: "Test aircraft".into(),
            aircraft_registration: None,
            started_at: "2026-07-01T00:00:00Z".into(),
            ended_at: Some("2026-07-01T01:00:00Z".into()),
            origin: "manual".into(),
            status: "completed".into(),
            sample_count: 0,
            pinned: true,
            plan_snapshot_json: None,
        })
        .unwrap();
    for (id, simulator_session_id) in [
        ("audio-pinned", Some("sim-pinned".to_owned())),
        ("audio-unpinned", None),
    ] {
        store
            .create_audio_session_record(
                &AudioSessionRecord {
                    id: id.into(),
                    simulator_session_id,
                    provider_id: "dev.wyrmgrid.fake-audio".into(),
                    capture_mode: "manual".into(),
                    started_at: "2026-07-01T00:00:00Z".into(),
                    ended_at: Some("2026-07-01T01:00:00Z".into()),
                    host_start_monotonic_ns: None,
                    status: "completed".into(),
                    media_availability: "available".into(),
                    total_media_bytes: 0,
                    deletion_requested_at: None,
                },
                &[AudioTrackRecord {
                    id: format!("track-{id}"),
                    session_id: id.into(),
                    source_id: "synthetic.microphone.primary".into(),
                    profile_id: "pilot_microphone_v1".into(),
                    source_role: "microphone_input".into(),
                    source_truth: "isolated".into(),
                    channel_count: 1,
                    sample_rate_hz: 48_000,
                    provider_start_monotonic_ns: 100,
                    packet_count: 0,
                    frame_count: 0,
                    last_packet_sequence: None,
                }],
            )
            .unwrap();
    }
    let candidates = store.list_audio_deletion_candidate_records().unwrap();
    assert_eq!(
        candidates
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        vec!["audio-unpinned"]
    );
}

#[test]
fn atlas_preferences_and_opt_in_view_round_trip() {
    let store = Store::open_in_memory().expect("store should initialize");
    let preferences = AtlasPreferencesRecord {
        automatic_sync_minutes: 60,
        daylight_visible: false,
        regions_visible: true,
        route_visible: true,
        fleet_visible: false,
        fbos_visible: true,
        airport_weather_visible: true,
        global_weather_visible: false,
        weather_coverage_visible: true,
        plugin_layers_visible: false,
        restore_last_view: true,
        last_longitude: None,
        last_latitude: None,
        last_zoom: None,
        last_bearing: None,
        last_pitch: None,
    };

    assert_eq!(store.load_atlas_preferences_record().unwrap(), None);
    store.save_atlas_preferences_record(&preferences).unwrap();
    assert!(
        store
            .save_atlas_view_record(151.2093, -91.0, 6.0, 12.0, 35.0)
            .is_err()
    );
    store
        .save_atlas_view_record(151.2093, -33.8688, 6.0, 12.0, 35.0)
        .unwrap();
    let saved = store
        .load_atlas_preferences_record()
        .unwrap()
        .expect("preferences should exist");
    assert_eq!(saved.automatic_sync_minutes, 60);
    assert!(!saved.daylight_visible);
    assert_eq!(saved.last_longitude, Some(151.2093));
    assert_eq!(saved.last_pitch, Some(35.0));

    let disabled = AtlasPreferencesRecord {
        restore_last_view: false,
        ..preferences
    };
    store.save_atlas_preferences_record(&disabled).unwrap();
    let cleared = store.load_atlas_preferences_record().unwrap().unwrap();
    assert_eq!(cleared.last_longitude, None);
    assert_eq!(cleared.last_latitude, None);
    assert_eq!(cleared.last_zoom, None);
    assert_eq!(cleared.last_bearing, None);
    assert_eq!(cleared.last_pitch, None);
}

#[test]
fn plugin_configuration_round_trips_without_manifest_or_permission_coupling() {
    let store = Store::open_in_memory().expect("store should initialize");
    let record = PluginConfigurationRecord {
        plugin_id: "org.wyrmgrid.provider.open-meteo".into(),
        setting_key: "weather_refresh_minutes".into(),
        value: "30".into(),
    };

    assert_eq!(
        store
            .load_plugin_configuration_record(&record.plugin_id, &record.setting_key)
            .unwrap(),
        None
    );
    store.save_plugin_configuration_record(&record).unwrap();
    assert_eq!(
        store
            .load_plugin_configuration_record(&record.plugin_id, &record.setting_key)
            .unwrap(),
        Some(record)
    );
}

#[test]
fn atlas_and_plugin_configuration_migration_preserves_version_16_preferences() {
    let connection = Connection::open_in_memory().expect("database should open");
    connection
        .execute_batch(INITIAL_SCHEMA)
        .expect("initial schema should apply");
    connection
        .execute_batch(PLUGIN_PREFERENCES_SCHEMA)
        .expect("version 16 schema should apply");
    connection
        .execute(
            "INSERT INTO plugin_preferences (
                plugin_id, scope_revision, start_with_wyrmgrid
             ) VALUES ('org.wyrmgrid.test', 'scope-v1', 1)",
            [],
        )
        .expect("existing preference should save");

    connection
        .execute_batch(ATLAS_AND_PLUGIN_CONFIGURATION_SCHEMA)
        .expect("version 17 schema should apply");
    connection
        .execute_batch(ATLAS_AND_PLUGIN_CONFIGURATION_SCHEMA)
        .expect("version 17 schema should be idempotent");

    let existing: (String, bool) = connection
        .query_row(
            "SELECT scope_revision, start_with_wyrmgrid
             FROM plugin_preferences WHERE plugin_id = 'org.wyrmgrid.test'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("existing preference should remain");
    assert_eq!(existing, ("scope-v1".into(), true));
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        17
    );
}

#[test]
fn aircraft_assignment_migration_preserves_existing_operation_revisions() {
    let connection = Connection::open_in_memory().expect("database should open");
    connection
        .execute_batch(INITIAL_SCHEMA)
        .expect("initial schema should apply");
    connection
        .execute_batch(FLIGHT_OPERATIONS_SCHEMA)
        .expect("flight-operation schema should apply");
    connection
        .execute(
            "INSERT INTO flight_operations (
                operation_id, created_at, current_revision, updated_at
             ) VALUES ('operation-a', '2026-07-20T00:00:00Z', 1, '2026-07-20T00:00:00Z')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO flight_operation_revisions (
                operation_id, revision, reason, created_at, snapshot_json
             ) VALUES ('operation-a', 1, 'initial', '2026-07-20T00:00:00Z', '{}')",
            [],
        )
        .unwrap();

    connection
        .execute_batch(FLIGHT_OPERATION_AIRCRAFT_ASSIGNMENTS_SCHEMA)
        .expect("aircraft-assignment schema should apply");
    connection
        .execute_batch(FLIGHT_OPERATION_AIRCRAFT_ASSIGNMENTS_SCHEMA)
        .expect("aircraft-assignment schema should be idempotent");

    let operation_revisions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM flight_operation_revisions",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(operation_revisions, 1);
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        19
    );
}

#[test]
fn plugin_startup_preferences_round_trip_and_delete() {
    let store = Store::open_in_memory().expect("store should initialize");
    let record = PluginPreferencesRecord {
        plugin_id: "org.wyrmgrid.test.weather".into(),
        scope_revision: "plugin:org.wyrmgrid.test.weather:v1".into(),
        start_with_wyrmgrid: true,
    };

    assert_eq!(
        store
            .load_plugin_preferences_record(&record.plugin_id)
            .unwrap(),
        None
    );
    store.save_plugin_preferences_record(&record).unwrap();
    assert_eq!(
        store
            .load_plugin_preferences_record(&record.plugin_id)
            .unwrap(),
        Some(record.clone())
    );

    let revised = PluginPreferencesRecord {
        scope_revision: "plugin:org.wyrmgrid.test.weather:v2".into(),
        start_with_wyrmgrid: false,
        ..record.clone()
    };
    store.save_plugin_preferences_record(&revised).unwrap();
    assert_eq!(
        store
            .load_plugin_preferences_record(&record.plugin_id)
            .unwrap(),
        Some(revised)
    );

    store
        .delete_plugin_preferences_record(&record.plugin_id)
        .unwrap();
    assert_eq!(
        store
            .load_plugin_preferences_record(&record.plugin_id)
            .unwrap(),
        None
    );
}

#[test]
fn migrates_existing_atlas_weather_preferences_idempotently() {
    let connection = Connection::open_in_memory().expect("database should open");
    connection
        .execute_batch(INITIAL_SCHEMA)
        .expect("initial schema should apply");
    connection
        .execute_batch(ATLAS_RENDERING_PREFERENCES_SCHEMA)
        .expect("profile schema should apply");
    connection
        .execute(
            "INSERT INTO atlas_rendering_preferences (
                singleton_id, weather_rendering_profile
             ) VALUES (1, 'compatibility')",
            [],
        )
        .expect("existing profile should save");

    connection
        .execute_batch(ATLAS_WEATHER_GRAPHICS_PREFERENCES_SCHEMA)
        .expect("graphics migration should apply");
    connection
        .execute_batch(ATLAS_WEATHER_GRAPHICS_PREFERENCES_SCHEMA)
        .expect("graphics migration should be idempotent");

    let migrated: (String, bool, bool, bool, bool, bool) = connection
        .query_row(
            "SELECT weather_rendering_profile,
                    weather_cloud_effects,
                    weather_precipitation_effects,
                    weather_lightning_effects,
                    weather_dust_effects,
                    reduce_weather_flashes
             FROM atlas_weather_graphics_preferences
             WHERE singleton_id = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("migrated graphics preference should be readable");
    assert_eq!(
        migrated,
        ("compatibility".into(), true, true, true, true, true)
    );
}

#[test]
fn flight_operation_revisions_are_append_only_and_active() {
    let store = Store::open_in_memory().expect("store should initialize");
    let initial = FlightOperationRevisionRecord {
        operation_id: "operation-a".into(),
        operation_created_at: "2026-07-17T06:00:00Z".into(),
        revision: 1,
        reason: "initial".into(),
        revision_created_at: "2026-07-17T06:00:00Z".into(),
        snapshot_json: "{\"revision\":1}".into(),
    };
    store.create_flight_operation_record(&initial).unwrap();
    let competing = FlightOperationRevisionRecord {
        operation_id: "operation-b".into(),
        ..initial.clone()
    };
    assert!(store.create_flight_operation_record(&competing).is_err());
    assert_eq!(
        store
            .load_active_flight_operation_revision_record()
            .unwrap(),
        Some(initial.clone())
    );

    let revised = FlightOperationRevisionRecord {
        revision: 2,
        reason: "job_changed".into(),
        revision_created_at: "2026-07-17T06:05:00Z".into(),
        snapshot_json: "{\"revision\":2}".into(),
        ..initial.clone()
    };
    store
        .append_flight_operation_revision_record(1, &revised)
        .unwrap();
    assert_eq!(
        store
            .load_active_flight_operation_revision_record()
            .unwrap(),
        Some(revised)
    );

    let connection = store.connection.lock().unwrap();
    let revision_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM flight_operation_revisions WHERE operation_id = ?1",
            ["operation-a"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revision_count, 2);
}

#[test]
fn flight_operation_revision_rejects_a_stale_expected_revision() {
    let store = Store::open_in_memory().expect("store should initialize");
    let initial = FlightOperationRevisionRecord {
        operation_id: "operation-a".into(),
        operation_created_at: "2026-07-17T06:00:00Z".into(),
        revision: 1,
        reason: "initial".into(),
        revision_created_at: "2026-07-17T06:00:00Z".into(),
        snapshot_json: "{}".into(),
    };
    store.create_flight_operation_record(&initial).unwrap();

    let stale = FlightOperationRevisionRecord {
        revision: 3,
        reason: "plan_changed".into(),
        revision_created_at: "2026-07-17T06:10:00Z".into(),
        ..initial
    };
    assert!(matches!(
        store.append_flight_operation_revision_record(2, &stale),
        Err(StorageError::InvalidRecord)
    ));
}

#[test]
fn aircraft_assignment_revisions_are_append_only_and_bound_to_the_active_operation() {
    let store = Store::open_in_memory().expect("store should initialize");
    let operation = FlightOperationRevisionRecord {
        operation_id: "operation-a".into(),
        operation_created_at: "2026-07-20T00:00:00Z".into(),
        revision: 1,
        reason: "initial".into(),
        revision_created_at: "2026-07-20T00:00:00Z".into(),
        snapshot_json: "{}".into(),
    };
    store.create_flight_operation_record(&operation).unwrap();
    assert_eq!(
        store
            .load_active_flight_operation_aircraft_assignment_revision_record()
            .unwrap(),
        None
    );

    let assigned = FlightOperationAircraftAssignmentRevisionRecord {
        operation_id: operation.operation_id.clone(),
        revision: 1,
        reason: "assigned".into(),
        reviewed_at: "2026-07-20T00:01:00Z".into(),
        snapshot_json: "{\"revision\":1}".into(),
    };
    store
        .append_flight_operation_aircraft_assignment_revision_record(None, &assigned)
        .unwrap();
    assert!(
        store
            .append_flight_operation_aircraft_assignment_revision_record(None, &assigned)
            .is_err()
    );

    let cleared = FlightOperationAircraftAssignmentRevisionRecord {
        revision: 2,
        reason: "cleared".into(),
        reviewed_at: "2026-07-20T00:02:00Z".into(),
        snapshot_json: "{\"revision\":2}".into(),
        ..assigned
    };
    store
        .append_flight_operation_aircraft_assignment_revision_record(Some(1), &cleared)
        .unwrap();
    assert_eq!(
        store
            .load_active_flight_operation_aircraft_assignment_revision_record()
            .unwrap(),
        Some(cleared)
    );
}

#[test]
fn authorization_grants_are_revision_bound_and_decisions_are_audited() {
    let store = Store::open_in_memory().expect("store should initialize");
    let subject_kind = "plugin";
    let subject_id = "org.example.weather";
    let first_revision = "plugin:1.0.0:on_air_company_read";
    let second_revision = "plugin:1.1.0:on_air_company_read|external_network";

    store
        .replace_authorization_grant_records(
            subject_kind,
            subject_id,
            first_revision,
            &["on_air_company_read".into()],
        )
        .expect("grant should save");
    assert_eq!(
        store
            .list_authorization_grant_records(subject_kind, subject_id, first_revision)
            .expect("matching revision should load"),
        vec!["on_air_company_read"]
    );
    assert!(
        store
            .list_authorization_grant_records(subject_kind, subject_id, second_revision)
            .expect("new revision should fail closed")
            .is_empty()
    );

    store
        .replace_authorization_grant_records(subject_kind, subject_id, second_revision, &[])
        .expect("revocation should save");
    let connection = store
        .connection
        .lock()
        .expect("storage connection should be available");
    let decisions: Vec<(String, i64)> = connection
        .prepare(
            "SELECT decision, capability_count FROM authorization_decisions
             WHERE subject_kind = ?1 AND subject_id = ?2 ORDER BY id ASC",
        )
        .expect("decision query should prepare")
        .query_map(params![subject_kind, subject_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("decisions should query")
        .collect::<Result<_, _>>()
        .expect("decisions should collect");
    assert_eq!(decisions, vec![("grant".into(), 1), ("revoke".into(), 0)]);
}

#[test]
fn authorization_decision_history_is_bounded() {
    let store = Store::open_in_memory().expect("store should initialize");
    for index in 0..4_100 {
        store
            .replace_authorization_grant_records(
                "plugin",
                "org.example.weather",
                &format!("plugin:{index}:on_air_company_read"),
                &["on_air_company_read".into()],
            )
            .expect("decision should save");
    }

    let connection = store
        .connection
        .lock()
        .expect("storage connection should be available");
    let (count, oldest_id): (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), MIN(id) FROM authorization_decisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("bounded decisions should be readable");
    assert_eq!(count, 4_096);
    assert_eq!(oldest_id, 5);
}

#[test]
fn persists_independent_display_preferences() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert!(
        store
            .load_display_preferences_record()
            .expect("display preferences should be readable")
            .is_none()
    );

    let preferences = DisplayPreferencesRecord {
        altitude_unit: "metres".into(),
        speed_unit: "knots".into(),
        weight_unit: "kilograms".into(),
        fuel_unit: "litres".into(),
        responsive_surfaces: false,
        weather_rendering_profile: "cinematic".into(),
        weather_cloud_effects: true,
        weather_precipitation_effects: false,
        weather_lightning_effects: true,
        weather_dust_effects: false,
        reduce_weather_flashes: false,
    };
    store
        .save_display_preferences_record(&preferences)
        .expect("display preferences should save");

    assert_eq!(
        store
            .load_display_preferences_record()
            .expect("display preferences should be readable"),
        Some(preferences.clone())
    );

    let connection = store
        .connection
        .lock()
        .expect("storage connection should be available");
    let legacy_profile: String = connection
        .query_row(
            "SELECT weather_rendering_profile
             FROM atlas_rendering_preferences WHERE singleton_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("the legacy fallback preference should be readable");
    let cinematic_profile: String = connection
        .query_row(
            "SELECT weather_rendering_profile
             FROM atlas_weather_graphics_preferences WHERE singleton_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("the authoritative graphics preference should be readable");
    assert_eq!(legacy_profile, "enhanced");
    assert_eq!(cinematic_profile, "cinematic");
}

#[test]
fn stores_and_forgets_provider_account_preferences_without_a_secret_column() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    let onair = OnAirAccountPreferencesRecord {
        company_id: "75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned(),
        connect_on_start: true,
    };
    let simbrief = SimBriefAccountPreferencesRecord {
        reference_kind: "pilot_id".to_owned(),
        reference: "1234567".to_owned(),
    };

    store
        .save_onair_account_preferences_record(&onair)
        .expect("OnAir account preferences should save");
    store
        .save_simbrief_account_preferences_record(&simbrief)
        .expect("SimBrief account preferences should save");

    assert_eq!(
        store.load_onair_account_preferences_record().unwrap(),
        Some(onair)
    );
    assert_eq!(
        store.load_simbrief_account_preferences_record().unwrap(),
        Some(simbrief)
    );

    store
        .delete_onair_account_preferences_record()
        .expect("OnAir account preferences should be removed");
    store
        .delete_simbrief_account_preferences_record()
        .expect("SimBrief account preferences should be removed");

    assert_eq!(store.load_onair_account_preferences_record().unwrap(), None);
    assert_eq!(
        store.load_simbrief_account_preferences_record().unwrap(),
        None
    );
}

#[test]
fn persists_simulator_provider_preferences_default_off() {
    let store = Store::open_in_memory().expect("store should initialize");
    assert!(store.load_simulator_preferences_record().unwrap().is_none());

    let preferences = SimulatorPreferencesRecord {
        selected_provider_id: Some("io.example.simulator".into()),
        start_with_wyrmgrid: true,
    };
    store
        .save_simulator_preferences_record(&preferences)
        .expect("preferences should save");

    assert_eq!(
        store.load_simulator_preferences_record().unwrap(),
        Some(preferences)
    );
}

#[test]
fn persists_and_deletes_bounded_simulator_recordings() {
    let store = Store::open_in_memory().expect("store should initialize");
    assert!(
        store
            .load_simulator_recording_preferences_record()
            .unwrap()
            .is_none()
    );
    store
        .save_simulator_recording_preferences_record(&SimulatorRecordingPreferencesRecord {
            retention_days: 30,
            automatic_start: true,
            automatic_stop: true,
            landing_settle_seconds: 45,
        })
        .unwrap();
    let preferences = store
        .load_simulator_recording_preferences_record()
        .unwrap()
        .unwrap();
    assert!(preferences.automatic_start);
    assert!(preferences.automatic_stop);
    assert_eq!(preferences.landing_settle_seconds, 45);

    let session = SimulatorSessionRecord {
        id: "session-1".into(),
        provider_id: "provider-1".into(),
        simulator_family: "MSFS_2024".into(),
        simulator_version: Some("1.0".into()),
        aircraft_title: "Cessna 172".into(),
        aircraft_registration: Some("VH-WYR".into()),
        started_at: "2026-07-15T00:00:00Z".into(),
        ended_at: None,
        origin: "manual".into(),
        status: "active".into(),
        sample_count: 0,
        pinned: false,
        plan_snapshot_json: None,
    };
    store.create_simulator_session_record(&session).unwrap();
    let sample = SimulatorSampleRecord {
        source_sequence: 1,
        observed_at: "2026-07-15T00:00:01Z".into(),
        simulation_time_utc: None,
        altitude_feet: 1_234.0,
        indicated_airspeed_knots: 90.0,
        true_airspeed_knots: 95.0,
        ground_speed_knots: 88.0,
        fuel_total_weight_pounds: Some(200.0),
        gross_weight_pounds: Some(2_100.0),
        pitch_degrees: 2.0,
        bank_degrees: -1.0,
        gap_before: false,
        latitude: Some(-33.8688),
        longitude: Some(151.2093),
        on_ground: Some(false),
        engines_running: Some(true),
        parking_brake_set: Some(false),
        paused: Some(false),
    };
    assert!(
        store
            .append_simulator_sample_record(&session.id, &sample)
            .unwrap()
    );
    assert!(
        !store
            .append_simulator_sample_record(&session.id, &sample)
            .unwrap()
    );

    let sessions = store.list_simulator_session_records(10).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].sample_count, 1);
    assert!(!sessions[0].pinned);
    assert_eq!(
        store
            .list_simulator_sample_records(&session.id, 600)
            .unwrap(),
        vec![sample]
    );
    store
        .append_simulator_session_event_record(
            &session.id,
            &SimulatorSessionEventRecord {
                id: 0,
                event_kind: "takeoff_confirmed".into(),
                observed_at: "2026-07-15T00:00:01Z".into(),
                source_sequence: Some(1),
                evidence_json: "{\"on_ground\":false}".into(),
            },
        )
        .unwrap();
    assert_eq!(
        store
            .list_simulator_session_event_records(&session.id)
            .unwrap()[0]
            .event_kind,
        "takeoff_confirmed"
    );

    store
        .finish_simulator_session_record(&session.id, "2026-07-15T00:10:00Z", "completed")
        .unwrap();
    assert!(
        store
            .set_simulator_session_pinned(&session.id, true)
            .unwrap()
    );
    assert_eq!(
        store
            .prune_simulator_session_records("2026-07-16T00:00:00Z")
            .unwrap(),
        0
    );
    assert!(store.list_simulator_session_records(10).unwrap()[0].pinned);
    store
        .set_simulator_session_pinned(&session.id, false)
        .unwrap();
    assert_eq!(
        store
            .prune_simulator_session_records("2026-07-16T00:00:00Z")
            .unwrap(),
        1
    );
    assert!(store.list_simulator_session_records(10).unwrap().is_empty());
}

#[test]
fn opening_storage_marks_abandoned_recordings_interrupted() {
    let store = Store::open_in_memory().expect("store should initialize");
    store
        .create_simulator_session_record(&SimulatorSessionRecord {
            id: "abandoned".into(),
            provider_id: "provider-1".into(),
            simulator_family: "MSFS_2024".into(),
            simulator_version: None,
            aircraft_title: "Cessna 172".into(),
            aircraft_registration: None,
            started_at: "2026-07-15T00:00:00Z".into(),
            ended_at: None,
            origin: "manual".into(),
            status: "active".into(),
            sample_count: 0,
            pinned: false,
            plan_snapshot_json: None,
        })
        .unwrap();
    store
        .interrupt_active_simulator_sessions("2026-07-15T00:01:00Z")
        .unwrap();
    let session = store.list_simulator_session_records(1).unwrap().remove(0);
    assert_eq!(session.status, "interrupted");
    assert_eq!(session.ended_at.as_deref(), Some("2026-07-15T00:01:00Z"));
}

#[test]
fn stores_and_restores_the_latest_snapshot() {
    let mut store = Store::open_in_memory().expect("in-memory database should open");
    let earlier = (Utc::now() - Duration::hours(2)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let latest = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    store
        .save_api_snapshot("fleet", "company-a", &earlier, "{\"version\":1}")
        .expect("earlier snapshot should save");
    store
        .save_api_snapshot("fleet", "company-a", &latest, "{\"version\":2}")
        .expect("latest snapshot should save");

    assert_eq!(
        store
            .latest_api_snapshot("fleet", Some("company-a"))
            .expect("snapshot lookup should succeed"),
        Some(ApiSnapshotRecord {
            resource_key: "company-a".into(),
            observed_at: latest,
            payload_json: "{\"version\":2}".into(),
        })
    );
}

#[test]
fn reads_bounded_history_and_the_snapshot_at_a_selected_time() {
    let mut store = Store::open_in_memory().expect("in-memory database should open");
    let latest_hour = Utc::now()
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .expect("current hour should be representable");
    let observations = [
        latest_hour - Duration::hours(3),
        latest_hour - Duration::hours(2),
        latest_hour - Duration::hours(1),
        latest_hour,
    ];
    for (index, observed_at) in observations.iter().enumerate() {
        store
            .save_api_snapshot(
                "fleet",
                "company-a",
                &observed_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                &format!("{{\"index\":{index}}}"),
            )
            .expect("snapshot should save");
    }

    let history = store
        .api_snapshot_history("fleet", "company-a", 2)
        .expect("history should be readable");
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0].observed_at,
        observations[2].to_rfc3339_opts(SecondsFormat::Secs, true)
    );
    assert_eq!(
        history[1].observed_at,
        observations[3].to_rfc3339_opts(SecondsFormat::Secs, true)
    );

    let selected_at = observations[2] + Duration::minutes(30);
    let selected = store
        .api_snapshot_at_or_before(
            "fleet",
            "company-a",
            &selected_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        )
        .expect("historical selection should be readable")
        .expect("a prior snapshot should exist");
    assert_eq!(
        selected.observed_at,
        observations[2].to_rfc3339_opts(SecondsFormat::Secs, true)
    );
}

#[test]
fn retains_hourly_recent_and_daily_older_snapshots_per_company() {
    let mut store = Store::open_in_memory().expect("in-memory database should open");
    let now = Utc::now();
    let completed_hour = (now - Duration::hours(1))
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .expect("completed hour should be representable");
    let older_day = (now - Duration::days(8))
        .with_hour(0)
        .and_then(|value| value.with_minute(0))
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .expect("older day should be representable");
    let observations = [
        completed_hour + Duration::minutes(10),
        completed_hour + Duration::minutes(40),
        now - Duration::hours(2),
        older_day + Duration::hours(3),
        older_day + Duration::hours(8),
        now - Duration::days(9),
    ];

    for (index, observed_at) in observations.iter().enumerate() {
        store
            .save_api_snapshot(
                "fleet",
                "company-a",
                &observed_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                &format!("{{\"index\":{index}}}"),
            )
            .expect("snapshot should save");
    }
    store
        .save_api_snapshot(
            "fleet",
            "company-b",
            &now.to_rfc3339_opts(SecondsFormat::Secs, true),
            "{\"company\":\"b\"}",
        )
        .expect("other company snapshot should save");

    let connection = store
        .connection
        .lock()
        .expect("storage connection should be available");
    let company_a_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM api_snapshots
             WHERE resource_kind = 'fleet' AND resource_key = 'company-a'",
            [],
            |row| row.get(0),
        )
        .expect("retained count should be available");
    let company_b_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM api_snapshots
             WHERE resource_kind = 'fleet' AND resource_key = 'company-b'",
            [],
            |row| row.get(0),
        )
        .expect("other company count should be available");

    assert_eq!(company_a_count, 4);
    assert_eq!(company_b_count, 1);
}

#[test]
fn persists_legal_acknowledgement_and_telemetry_choice() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert!(
        store
            .load_legal_preferences_record()
            .expect("preferences should be readable")
            .is_none()
    );

    store
        .save_legal_preferences_record("terms-v1", "privacy-v1", true)
        .expect("preferences should be saved");
    let preferences = store
        .load_legal_preferences_record()
        .expect("preferences should be readable")
        .expect("preferences should exist");
    assert_eq!(preferences.terms_version, "terms-v1");
    assert_eq!(preferences.privacy_notice_version, "privacy-v1");
    assert!(preferences.telemetry_enabled);
    assert!(!preferences.acknowledged_at.is_empty());
}

#[test]
fn persists_custom_themes_and_the_selected_theme() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert!(
        store
            .load_theme_preferences_record()
            .expect("theme preference should be readable")
            .is_none()
    );

    store
        .save_custom_theme_record("night-flight", "{\"schema_version\":1}")
        .expect("custom theme should save");
    store
        .save_selected_theme_record("night-flight")
        .expect("theme selection should save");

    let themes = store
        .list_custom_theme_records()
        .expect("custom themes should be readable");
    assert_eq!(themes.len(), 1);
    assert_eq!(themes[0].theme_id, "night-flight");
    assert_eq!(themes[0].manifest_json, "{\"schema_version\":1}");
    assert!(themes[0].imported_at.ends_with('Z'));
    assert!(themes[0].updated_at.ends_with('Z'));

    {
        let connection = store.connection.lock().expect("database should lock");
        connection
            .execute(
                "UPDATE custom_themes
                 SET imported_at = '2020-01-02 03:04:05',
                     updated_at = '2020-01-02 03:04:05'
                 WHERE theme_id = 'night-flight'",
                [],
            )
            .expect("theme timestamps should be adjustable for the fixture");
    }
    store
        .save_custom_theme_record("night-flight", "{\"schema_version\":1,\"revision\":2}")
        .expect("custom theme revision should save");
    let revised = store
        .list_custom_theme_records()
        .expect("custom themes should be readable");
    assert_eq!(revised[0].imported_at, "2020-01-02T03:04:05Z");
    assert_eq!(
        revised[0].manifest_json,
        "{\"schema_version\":1,\"revision\":2}"
    );
    assert_ne!(revised[0].updated_at, revised[0].imported_at);
    assert_eq!(
        store
            .load_theme_preferences_record()
            .expect("theme preference should be readable"),
        Some(ThemePreferencesRecord {
            selected_theme_id: "night-flight".into(),
        })
    );
}

#[test]
fn deletes_a_custom_theme_and_resets_its_selection_atomically() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    store
        .save_custom_theme_record("night-flight", "{\"schema_version\":1}")
        .expect("custom theme should save");
    store
        .save_selected_theme_record("night-flight")
        .expect("theme selection should save");

    store
        .delete_custom_theme_record("night-flight", "wyrmgrid-classic")
        .expect("custom theme should be deleted");

    assert!(
        store
            .list_custom_theme_records()
            .expect("custom themes should be readable")
            .is_empty()
    );
    assert_eq!(
        store
            .load_theme_preferences_record()
            .expect("theme preference should be readable"),
        Some(ThemePreferencesRecord {
            selected_theme_id: "wyrmgrid-classic".into(),
        })
    );
}

#[test]
fn persists_custom_language_packs_and_the_selected_pack() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert!(
        store
            .load_language_preferences_record()
            .expect("language preference should be readable")
            .is_none()
    );

    store
        .save_custom_language_pack_record("community-fr", "{\"schema_version\":1}")
        .expect("custom language pack should save");
    store
        .save_selected_language_pack_record("community-fr")
        .expect("language selection should save");

    assert_eq!(
        store
            .list_custom_language_pack_records()
            .expect("custom language packs should be readable"),
        vec![CustomLanguagePackRecord {
            pack_id: "community-fr".into(),
            manifest_json: "{\"schema_version\":1}".into(),
        }]
    );
    assert_eq!(
        store
            .load_language_preferences_record()
            .expect("language preference should be readable"),
        Some(LanguagePreferencesRecord {
            selected_language_pack_id: "community-fr".into(),
        })
    );
}

#[test]
fn persists_deny_by_default_plugin_permission_grants() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    let plugin_id = "org.wyrmgrid.example.fleet-locations";
    assert!(
        store
            .list_plugin_permission_records(plugin_id)
            .expect("empty grants should be readable")
            .is_empty()
    );

    store
        .replace_plugin_permission_records(
            plugin_id,
            &["map_layers_publish".into(), "on_air_fleet_read".into()],
        )
        .expect("grants should persist");
    assert_eq!(
        store
            .list_plugin_permission_records(plugin_id)
            .expect("grants should be readable"),
        vec!["map_layers_publish", "on_air_fleet_read"]
    );

    store
        .replace_plugin_permission_records(plugin_id, &[])
        .expect("grants should be revocable");
    assert!(
        store
            .list_plugin_permission_records(plugin_id)
            .expect("revoked grants should be readable")
            .is_empty()
    );
}

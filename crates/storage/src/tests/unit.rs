use super::*;
use chrono::{Duration, SecondsFormat, Timelike, Utc};

#[test]
fn initializes_the_database_schema() {
    let store = Store::open_in_memory().expect("in-memory database should open");
    assert_eq!(
        store.schema_version().expect("version should be readable"),
        6
    );
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
    };
    store
        .save_display_preferences_record(&preferences)
        .expect("display preferences should save");

    assert_eq!(
        store
            .load_display_preferences_record()
            .expect("display preferences should be readable"),
        Some(preferences)
    );
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

    assert_eq!(
        store
            .list_custom_theme_records()
            .expect("custom themes should be readable"),
        vec![CustomThemeRecord {
            theme_id: "night-flight".into(),
            manifest_json: "{\"schema_version\":1}".into(),
        }]
    );
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

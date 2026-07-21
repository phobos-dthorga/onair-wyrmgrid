use std::fs;

use tempfile::tempdir;

use super::*;
use crate::{
    AudioSegmentRecord, AudioSessionRecord, AudioTrackRecord, DisplayPreferencesRecord,
    OnAirAccountPreferencesRecord, SimBriefAccountPreferencesRecord,
};

const BACKUP_PASSWORD: &str = "a deliberate migration password";

#[test]
fn persistent_database_is_encrypted_and_requires_the_device_key() {
    let directory = tempdir().expect("temporary directory should exist");
    let path = directory.path().join("wyrmgrid.db");
    let key = DatabaseKey::from_bytes([7; 32]);
    let wrong_key = DatabaseKey::from_bytes([8; 32]);

    let store = Store::open(&path, &key).expect("encrypted database should open");
    assert_eq!(store.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
    drop(store);

    let header = fs::read(&path).expect("encrypted database should be readable as bytes");
    assert_ne!(&header[..16], b"SQLite format 3\0");
    assert!(Store::open(&path, &wrong_key).is_err());
    assert!(Store::open(&path, &key).is_ok());
    assert_eq!(format!("{key:?}"), "DatabaseKey([REDACTED])");
}

#[test]
fn portable_backup_preserves_audio_metadata_but_marks_external_media_omitted() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let backup_path = directory.path().join("audio-omitted.wyrmbackup");
    let key = DatabaseKey::from_bytes([17; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    let session = AudioSessionRecord {
        id: "audio-session-backup".into(),
        simulator_session_id: None,
        provider_id: "dev.wyrmgrid.fake-audio".into(),
        capture_mode: "manual".into(),
        started_at: "2026-07-20T00:00:00Z".into(),
        ended_at: Some("2026-07-20T00:01:00Z".into()),
        host_start_monotonic_ns: None,
        status: "completed".into(),
        media_availability: "available".into(),
        total_media_bytes: 0,
        deletion_requested_at: None,
    };
    let track = AudioTrackRecord {
        id: "audio-track-backup".into(),
        session_id: session.id.clone(),
        source_id: "synthetic.microphone.primary".into(),
        profile_id: "pilot_microphone_v1".into(),
        codec_provider_id: "dev.wyrmgrid.opus".into(),
        codec_provider_version: "0.3.1".into(),
        codec_id: "opus".into(),
        codec_media_type: "audio/opus".into(),
        source_role: "microphone_input".into(),
        source_truth: "isolated".into(),
        channel_count: 1,
        sample_rate_hz: 48_000,
        provider_start_monotonic_ns: 100,
        packet_count: 0,
        frame_count: 0,
        last_packet_sequence: None,
    };
    store
        .create_audio_session_record(&session, std::slice::from_ref(&track))
        .unwrap();
    store
        .complete_audio_segment_record(
            &AudioSegmentRecord {
                track_id: track.id.clone(),
                segment_index: 0,
                storage_key: "0123456789abcdef0123456789abcdef".into(),
                first_frame: 0,
                frame_count: 960,
                packet_count: 1,
                encrypted_bytes: 96,
                envelope_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .into(),
                envelope_version: 1,
                key_version: 1,
                state: "complete".into(),
                created_at: "2026-07-20T00:00:00Z".into(),
                deletion_requested_at: None,
            },
            1,
        )
        .unwrap();
    store
        .export_portable_backup(
            &backup_path,
            BACKUP_PASSWORD,
            "2026-07-20T00:02:00Z",
            "0.2.0",
        )
        .unwrap();
    store
        .prepare_portable_restore(&backup_path, BACKUP_PASSWORD, &key)
        .unwrap();
    drop(store);

    let restored = Store::open(&database_path, &key).unwrap();
    let restored_session = restored.list_audio_session_records().unwrap().remove(0);
    assert_eq!(restored_session.media_availability, "not_in_backup");
    let restored_segment = restored
        .list_audio_segment_records(&track.id)
        .unwrap()
        .remove(0);
    assert_eq!(restored_segment.state, "unavailable");
}

#[test]
fn plaintext_database_is_never_opened_or_rewritten_as_encrypted_storage() {
    let directory = tempdir().expect("temporary directory should exist");
    let path = directory.path().join("wyrmgrid.db");
    let plaintext = Connection::open(&path).expect("plaintext fixture should open");
    plaintext
        .execute_batch("CREATE TABLE legacy_marker (value TEXT NOT NULL);")
        .expect("plaintext fixture should be initialized");
    drop(plaintext);
    let original = fs::read(&path).expect("plaintext fixture should be readable");

    assert!(Store::open(&path, &DatabaseKey::from_bytes([9; 32])).is_err());
    assert_eq!(
        fs::read(&path).expect("plaintext fixture should remain readable"),
        original
    );
    assert_eq!(&original[..16], b"SQLite format 3\0");
}

#[test]
fn portable_backup_round_trip_replaces_data_only_after_reopen() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let backup_path = directory.path().join("journey.wyrmgrid-backup");
    let key = DatabaseKey::from_bytes([11; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    let original = preferences("metres", "litres");
    let changed = preferences("feet", "pounds");
    store
        .save_display_preferences_record(&original)
        .expect("original preferences should save");
    let onair_account = OnAirAccountPreferencesRecord {
        company_id: "75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned(),
        connect_on_start: true,
    };
    let simbrief_account = SimBriefAccountPreferencesRecord {
        reference_kind: "pilot_id".to_owned(),
        reference: "1234567".to_owned(),
    };
    store
        .save_onair_account_preferences_record(&onair_account)
        .expect("OnAir metadata should save");
    store
        .save_simbrief_account_preferences_record(&simbrief_account)
        .expect("SimBrief metadata should save");

    let exported = store
        .export_portable_backup(
            &backup_path,
            BACKUP_PASSWORD,
            "2026-07-15T00:00:00Z",
            "0.1.0",
        )
        .expect("portable backup should export");
    assert_eq!(exported.format_version, PORTABLE_BACKUP_FORMAT_VERSION);
    let backup_header = fs::read(&backup_path).expect("backup should be readable as bytes");
    assert_ne!(&backup_header[..16], b"SQLite format 3\0");

    store
        .save_display_preferences_record(&changed)
        .expect("changed preferences should save");
    store
        .delete_onair_account_preferences_record()
        .expect("OnAir metadata should change after backup");
    store
        .delete_simbrief_account_preferences_record()
        .expect("SimBrief metadata should change after backup");
    let restore = store
        .prepare_portable_restore(&backup_path, BACKUP_PASSWORD, &key)
        .expect("restore should prepare");
    assert!(restore.restart_required);
    assert_eq!(
        store.load_display_preferences_record().unwrap(),
        Some(changed)
    );
    drop(store);

    let restored = Store::open(&database_path, &key).expect("pending restore should activate");
    assert_eq!(
        restored.load_display_preferences_record().unwrap(),
        Some(original)
    );
    assert_eq!(
        restored.load_onair_account_preferences_record().unwrap(),
        Some(onair_account)
    );
    assert_eq!(
        restored.load_simbrief_account_preferences_record().unwrap(),
        Some(simbrief_account)
    );
    assert!(!pending_path(&database_path).exists());
    assert!(!rollback_path(&database_path).exists());
}

#[test]
fn wrong_backup_password_does_not_create_a_pending_restore() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let backup_path = directory.path().join("journey.wyrmgrid-backup");
    let key = DatabaseKey::from_bytes([19; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    store
        .export_portable_backup(
            &backup_path,
            BACKUP_PASSWORD,
            "2026-07-15T00:00:00Z",
            "0.1.0",
        )
        .expect("portable backup should export");

    assert!(matches!(
        store.prepare_portable_restore(&backup_path, "the wrong password", &key),
        Err(StorageError::InvalidPortableBackup)
    ));
    assert!(!pending_path(&database_path).exists());
}

#[test]
fn unsupported_backup_format_does_not_create_a_pending_restore() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let backup_path = directory.path().join("future.wyrmgrid-backup");
    let key = DatabaseKey::from_bytes([21; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    store
        .export_portable_backup(
            &backup_path,
            BACKUP_PASSWORD,
            "2026-07-15T00:00:00Z",
            "0.1.0",
        )
        .expect("portable backup should export");

    let connection = Connection::open(&backup_path).expect("backup should open");
    apply_key(&connection, BACKUP_PASSWORD.as_bytes()).expect("backup key should apply");
    connection
        .execute("UPDATE wyrmgrid_backup_manifest SET format_version = 2", [])
        .expect("fixture format should update");
    drop(connection);

    assert!(matches!(
        store.prepare_portable_restore(&backup_path, BACKUP_PASSWORD, &key),
        Err(StorageError::InvalidPortableBackup)
    ));
    assert!(!pending_path(&database_path).exists());
}

#[test]
fn failed_pending_restore_rolls_back_to_the_last_database() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let key = DatabaseKey::from_bytes([23; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    let original = preferences("metres", "litres");
    store
        .save_display_preferences_record(&original)
        .expect("preferences should save");
    drop(store);
    fs::write(pending_path(&database_path), b"not a database")
        .expect("invalid pending file should save");

    let reopened = Store::open(&database_path, &key).expect("old database should roll back");
    assert_eq!(
        reopened.load_display_preferences_record().unwrap(),
        Some(original)
    );
    assert!(!rollback_path(&database_path).exists());
}

#[test]
fn existing_destination_and_active_source_fail_closed() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let destination = directory.path().join("existing.wyrmgrid-backup");
    let key = DatabaseKey::from_bytes([29; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    fs::write(&destination, b"keep me").expect("existing destination should save");

    assert!(matches!(
        store.export_portable_backup(
            &destination,
            BACKUP_PASSWORD,
            "2026-07-15T00:00:00Z",
            "0.1.0"
        ),
        Err(StorageError::BackupDestinationExists)
    ));
    assert_eq!(fs::read(&destination).unwrap(), b"keep me");
    assert!(matches!(
        store.prepare_portable_restore(&database_path, BACKUP_PASSWORD, &key),
        Err(StorageError::RestoreSourceIsActiveDatabase)
    ));
}

#[test]
fn pending_local_data_reset_removes_only_database_sets_and_reopens_empty() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    let key = DatabaseKey::from_bytes([41; 32]);
    let store = Store::open(&database_path, &key).expect("database should open");
    store
        .save_display_preferences_record(&preferences("metres", "litres"))
        .expect("preferences should save");
    store
        .prepare_local_data_reset()
        .expect("reset should be scheduled");
    drop(store);

    for path in [
        companion_path(&database_path, "-wal"),
        companion_path(&database_path, "-shm"),
        pending_path(&database_path),
        companion_path(&pending_path(&database_path), "-wal"),
        rollback_path(&database_path),
        companion_path(&rollback_path(&database_path), "-shm"),
    ] {
        fs::write(path, b"database companion").expect("companion should save");
    }
    let backup = directory.path().join("saved.wyrmbackup");
    let plugin = directory.path().join("plugins").join("example.py");
    fs::create_dir_all(plugin.parent().unwrap()).expect("plugin directory should create");
    fs::write(&backup, b"portable backup").expect("backup should save");
    fs::write(&plugin, b"plugin source").expect("plugin should save");

    assert!(apply_pending_local_data_reset(&database_path).unwrap());
    assert!(!database_path.exists());
    assert!(!pending_path(&database_path).exists());
    assert!(!rollback_path(&database_path).exists());
    assert!(!reset_marker_path(&database_path).exists());
    assert_eq!(fs::read(&backup).unwrap(), b"portable backup");
    assert_eq!(fs::read(&plugin).unwrap(), b"plugin source");
    assert!(!apply_pending_local_data_reset(&database_path).unwrap());

    let reopened = Store::open(&database_path, &key).expect("empty database should reopen");
    assert_eq!(reopened.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
    assert_eq!(reopened.load_display_preferences_record().unwrap(), None);
}

#[test]
fn invalid_local_data_reset_marker_fails_without_deleting_data() {
    let directory = tempdir().expect("temporary directory should exist");
    let database_path = directory.path().join("wyrmgrid.db");
    fs::write(&database_path, b"keep database").expect("database fixture should save");
    fs::write(reset_marker_path(&database_path), b"invalid reset request")
        .expect("invalid marker should save");

    assert!(matches!(
        apply_pending_local_data_reset(&database_path),
        Err(StorageError::InvalidRecord)
    ));
    assert_eq!(fs::read(&database_path).unwrap(), b"keep database");
    assert!(reset_marker_path(&database_path).exists());
}

#[test]
fn in_memory_store_cannot_schedule_a_local_data_reset() {
    let store = Store::open_in_memory().expect("store should open");
    assert!(matches!(
        store.prepare_local_data_reset(),
        Err(StorageError::PersistentStorageRequired)
    ));
}

fn preferences(altitude_unit: &str, fuel_unit: &str) -> DisplayPreferencesRecord {
    DisplayPreferencesRecord {
        altitude_unit: altitude_unit.into(),
        speed_unit: "knots".into(),
        weight_unit: "kilograms".into(),
        fuel_unit: fuel_unit.into(),
        responsive_surfaces: true,
        weather_rendering_profile: "enhanced".into(),
        weather_cloud_effects: true,
        weather_precipitation_effects: true,
        weather_lightning_effects: true,
        weather_dust_effects: true,
        reduce_weather_flashes: true,
    }
}

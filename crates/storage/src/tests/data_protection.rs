use std::fs;

use tempfile::tempdir;

use super::*;
use crate::DisplayPreferencesRecord;

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

fn preferences(altitude_unit: &str, fuel_unit: &str) -> DisplayPreferencesRecord {
    DisplayPreferencesRecord {
        altitude_unit: altitude_unit.into(),
        speed_unit: "knots".into(),
        weight_unit: "kilograms".into(),
        fuel_unit: fuel_unit.into(),
    }
}

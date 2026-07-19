use std::ffi::c_void;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, ffi, params};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{CURRENT_SCHEMA_VERSION, StorageError, Store};

pub const PORTABLE_BACKUP_FORMAT_VERSION: u32 = 1;
const DATABASE_KEY_BYTES: usize = 32;
const LOCAL_DATA_RESET_MARKER: &[u8] = b"wyrmgrid-local-data-reset-v1\n";
const LOCAL_DATA_RESET_SUFFIX: &str = ".reset-pending";
const LOCAL_DATA_RESET_PARTIAL_SUFFIX: &str = ".reset-pending.partial";
const WYRMGRID_APPLICATION_ID: i64 = 1_465_471_565;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DatabaseKey([u8; DATABASE_KEY_BYTES]);

impl DatabaseKey {
    pub fn from_bytes(bytes: [u8; DATABASE_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, StorageError> {
        let bytes = <[u8; DATABASE_KEY_BYTES]>::try_from(bytes)
            .map_err(|_| StorageError::InvalidDatabaseKey)?;
        Ok(Self(bytes))
    }

    pub fn expose(&self) -> &[u8; DATABASE_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for DatabaseKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableBackupRecord {
    pub format_version: u32,
    pub schema_version: i64,
    pub created_at: String,
    pub application_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableRestoreRecord {
    pub format_version: u32,
    pub schema_version: i64,
    pub created_at: String,
    pub application_version: String,
    pub restart_required: bool,
}

impl Store {
    pub fn portable_backup_pending_restore(&self) -> Result<bool, StorageError> {
        let path = self
            .path
            .as_ref()
            .ok_or(StorageError::PersistentStorageRequired)?;
        Ok(pending_path(path).exists())
    }

    pub fn export_portable_backup(
        &self,
        destination: impl AsRef<Path>,
        passphrase: &str,
        created_at: &str,
        application_version: &str,
    ) -> Result<PortableBackupRecord, StorageError> {
        let destination = destination.as_ref();
        let source_path = self
            .path
            .as_ref()
            .ok_or(StorageError::PersistentStorageRequired)?;
        if destination.exists() {
            return Err(StorageError::BackupDestinationExists);
        }
        if paths_refer_to_same_file(source_path, destination)? {
            return Err(StorageError::BackupDestinationExists);
        }

        let partial = sibling_path(destination, ".wyrmgrid-partial");
        remove_file_if_present(&partial)?;
        let export_result =
            self.export_to_attached_backup(&partial, passphrase, created_at, application_version);
        if let Err(error) = export_result {
            let _ = remove_file_if_present(&partial);
            return Err(error);
        }

        let record = validate_portable_backup(&partial, passphrase)?;
        fs::rename(&partial, destination).map_err(StorageError::FileOperation)?;
        Ok(record)
    }

    pub fn prepare_portable_restore(
        &self,
        source: impl AsRef<Path>,
        passphrase: &str,
        device_key: &DatabaseKey,
    ) -> Result<PortableRestoreRecord, StorageError> {
        let source = source.as_ref();
        let active_path = self
            .path
            .as_ref()
            .ok_or(StorageError::PersistentStorageRequired)?;
        if paths_refer_to_same_file(active_path, source)? {
            return Err(StorageError::RestoreSourceIsActiveDatabase);
        }
        let backup = validate_portable_backup(source, passphrase)?;
        let pending = pending_path(active_path);
        let partial = sibling_path(&pending, ".partial");
        remove_file_if_present(&partial)?;

        let prepare_result =
            export_backup_to_device_database(source, passphrase, &partial, device_key);
        if let Err(error) = prepare_result {
            let _ = remove_file_if_present(&partial);
            return Err(error);
        }
        validate_device_database(&partial, device_key)?;
        remove_file_if_present(&pending)?;
        fs::rename(&partial, &pending).map_err(StorageError::FileOperation)?;

        Ok(PortableRestoreRecord {
            format_version: backup.format_version,
            schema_version: backup.schema_version,
            created_at: backup.created_at,
            application_version: backup.application_version,
            restart_required: true,
        })
    }

    fn export_to_attached_backup(
        &self,
        destination: &Path,
        passphrase: &str,
        created_at: &str,
        application_version: &str,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;
        connection.execute(
            "ATTACH DATABASE ?1 AS portable KEY ?2",
            params![destination.to_string_lossy(), passphrase],
        )?;
        let operation = (|| {
            connection.query_row("SELECT sqlcipher_export('portable')", [], |_| Ok(()))?;
            connection.execute_batch(
                "CREATE TABLE portable.wyrmgrid_backup_manifest (
                    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
                    format_version INTEGER NOT NULL,
                    schema_version INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    application_version TEXT NOT NULL
                );",
            )?;
            connection.execute(
                "INSERT INTO portable.wyrmgrid_backup_manifest
                    (singleton_id, format_version, schema_version, created_at, application_version)
                 VALUES (1, ?1, ?2, ?3, ?4)",
                params![
                    PORTABLE_BACKUP_FORMAT_VERSION,
                    CURRENT_SCHEMA_VERSION,
                    created_at,
                    application_version
                ],
            )?;
            connection.execute_batch("PRAGMA portable.application_id = 1465471565;")?;
            connection.pragma_update(Some("portable"), "user_version", CURRENT_SCHEMA_VERSION)?;
            Ok::<(), StorageError>(())
        })();
        let detach = connection.execute_batch("DETACH DATABASE portable;");
        operation.and_then(|_| detach.map_err(StorageError::from))
    }
}

pub fn encrypted_database_state_exists(active_path: impl AsRef<Path>) -> bool {
    let active_path = active_path.as_ref();
    active_path.exists()
        || pending_path(active_path).exists()
        || rollback_path(active_path).exists()
}

pub(crate) fn prepare_local_data_reset(active_path: &Path) -> Result<(), StorageError> {
    let marker = reset_marker_path(active_path);
    if marker.exists() {
        return validate_reset_marker(&marker);
    }
    let partial = sibling_path(active_path, LOCAL_DATA_RESET_PARTIAL_SUFFIX);
    remove_file_if_present(&partial)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(StorageError::FileOperation)?;
    file.write_all(LOCAL_DATA_RESET_MARKER)
        .and_then(|_| file.sync_all())
        .map_err(StorageError::FileOperation)?;
    fs::rename(&partial, marker).map_err(StorageError::FileOperation)
}

pub fn apply_pending_local_data_reset(active_path: impl AsRef<Path>) -> Result<bool, StorageError> {
    let active_path = active_path.as_ref();
    let marker = reset_marker_path(active_path);
    if !marker.exists() {
        return Ok(false);
    }
    validate_reset_marker(&marker)?;
    remove_database_set(&pending_path(active_path))?;
    remove_database_set(&rollback_path(active_path))?;
    remove_database_set(active_path)?;
    remove_file_if_present(&sibling_path(active_path, LOCAL_DATA_RESET_PARTIAL_SUFFIX))?;
    remove_file_if_present(&marker)?;
    Ok(true)
}

fn validate_reset_marker(path: &Path) -> Result<(), StorageError> {
    let marker = fs::read(path).map_err(StorageError::FileOperation)?;
    if marker == LOCAL_DATA_RESET_MARKER {
        Ok(())
    } else {
        Err(StorageError::InvalidRecord)
    }
}

fn reset_marker_path(active_path: &Path) -> PathBuf {
    sibling_path(active_path, LOCAL_DATA_RESET_SUFFIX)
}

pub(crate) fn open_encrypted_connection(
    path: &Path,
    key: &DatabaseKey,
) -> Result<Connection, StorageError> {
    let connection = Connection::open(path)?;
    disable_cipher_logging(&connection)?;
    apply_key(&connection, key.expose())?;
    verify_cipher_available(&connection)?;
    connection.query_row("SELECT COUNT(*) FROM sqlite_master", [], |_| Ok(()))?;
    Ok(connection)
}

pub(crate) fn mark_wyrmgrid_database(connection: &Connection) -> Result<(), StorageError> {
    let application_id: i64 =
        connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if application_id != 0 && application_id != WYRMGRID_APPLICATION_ID {
        return Err(StorageError::InvalidRecord);
    }
    connection.execute_batch("PRAGMA application_id = 1465471565;")?;
    connection.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}

pub(crate) fn activate_pending_restore(
    active_path: &Path,
    key: &DatabaseKey,
) -> Result<(), StorageError> {
    let pending = pending_path(active_path);
    let rollback = rollback_path(active_path);

    if pending.exists() {
        if rollback.exists() && active_path.exists() {
            return Err(StorageError::InvalidPortableBackup);
        }
        if !rollback.exists() && active_path.exists() {
            rename_database_set(active_path, &rollback)?;
        }
        fs::rename(&pending, active_path).map_err(StorageError::FileOperation)?;
    }

    if rollback.exists() {
        match validate_device_database(active_path, key) {
            Ok(()) => return Ok(()),
            Err(_) => {
                remove_database_set(active_path)?;
                rename_database_set(&rollback, active_path)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn finish_pending_restore(active_path: &Path) -> Result<(), StorageError> {
    remove_database_set(&rollback_path(active_path))
}

fn validate_device_database(path: &Path, key: &DatabaseKey) -> Result<(), StorageError> {
    let connection = open_encrypted_connection(path, key)?;
    let application_id: i64 =
        connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    let schema_version: i64 =
        connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    if application_id != WYRMGRID_APPLICATION_ID || schema_version > CURRENT_SCHEMA_VERSION {
        return Err(StorageError::InvalidPortableBackup);
    }
    verify_database_integrity(&connection)
}

fn validate_portable_backup(
    path: &Path,
    passphrase: &str,
) -> Result<PortableBackupRecord, StorageError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| StorageError::InvalidPortableBackup)?;
    disable_cipher_logging(&connection).map_err(|_| StorageError::InvalidPortableBackup)?;
    apply_key(&connection, passphrase.as_bytes())
        .map_err(|_| StorageError::InvalidPortableBackup)?;
    verify_cipher_available(&connection).map_err(|_| StorageError::InvalidPortableBackup)?;
    verify_database_integrity(&connection).map_err(|_| StorageError::InvalidPortableBackup)?;
    let application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(|_| StorageError::InvalidPortableBackup)?;
    let record = connection
        .query_row(
            "SELECT format_version, schema_version, created_at, application_version
             FROM wyrmgrid_backup_manifest WHERE singleton_id = 1",
            [],
            |row| {
                Ok(PortableBackupRecord {
                    format_version: row.get(0)?,
                    schema_version: row.get(1)?,
                    created_at: row.get(2)?,
                    application_version: row.get(3)?,
                })
            },
        )
        .map_err(|_| StorageError::InvalidPortableBackup)?;
    if application_id != WYRMGRID_APPLICATION_ID
        || record.format_version != PORTABLE_BACKUP_FORMAT_VERSION
        || record.schema_version > CURRENT_SCHEMA_VERSION
    {
        return Err(StorageError::InvalidPortableBackup);
    }
    Ok(record)
}

fn export_backup_to_device_database(
    source: &Path,
    passphrase: &str,
    destination: &Path,
    key: &DatabaseKey,
) -> Result<(), StorageError> {
    let connection = Connection::open(source).map_err(|_| StorageError::InvalidPortableBackup)?;
    disable_cipher_logging(&connection).map_err(|_| StorageError::InvalidPortableBackup)?;
    apply_key(&connection, passphrase.as_bytes())
        .map_err(|_| StorageError::InvalidPortableBackup)?;
    connection.execute(
        "ATTACH DATABASE ?1 AS restored KEY ?2",
        params![destination.to_string_lossy(), key.expose().as_slice()],
    )?;
    let operation = (|| {
        connection.query_row("SELECT sqlcipher_export('restored')", [], |_| Ok(()))?;
        connection.execute_batch(
            "DROP TABLE restored.wyrmgrid_backup_manifest;
             PRAGMA restored.application_id = 1465471565;",
        )?;
        connection.pragma_update(Some("restored"), "user_version", CURRENT_SCHEMA_VERSION)?;
        Ok::<(), StorageError>(())
    })();
    let detach = connection.execute_batch("DETACH DATABASE restored;");
    operation.and_then(|_| detach.map_err(StorageError::from))
}

fn apply_key(connection: &Connection, key: &[u8]) -> Result<(), StorageError> {
    if key.is_empty() || key.len() > i32::MAX as usize {
        return Err(StorageError::InvalidDatabaseKey);
    }
    let result = unsafe {
        ffi::sqlite3_key(
            connection.handle(),
            key.as_ptr().cast::<c_void>(),
            key.len() as i32,
        )
    };
    if result == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(StorageError::InvalidDatabaseKey)
    }
}

fn verify_cipher_available(connection: &Connection) -> Result<(), StorageError> {
    let version = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get::<_, String>(0))
        .optional()?;
    version
        .filter(|version| !version.trim().is_empty())
        .map(|_| ())
        .ok_or(StorageError::EncryptionUnavailable)
}

fn disable_cipher_logging(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch("PRAGMA cipher_log_level = NONE;")?;
    Ok(())
}

fn verify_database_integrity(connection: &Connection) -> Result<(), StorageError> {
    let cipher_error = connection
        .query_row("PRAGMA cipher_integrity_check", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    if cipher_error.is_some() {
        return Err(StorageError::InvalidPortableBackup);
    }
    let quick_check: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if quick_check == "ok" {
        Ok(())
    } else {
        Err(StorageError::InvalidPortableBackup)
    }
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> Result<bool, StorageError> {
    let left = absolute_path(left)?;
    let right = absolute_path(right)?;
    Ok(left == right)
}

fn absolute_path(path: &Path) -> Result<PathBuf, StorageError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(path))
            .map_err(StorageError::FileOperation)
    }
}

fn pending_path(active: &Path) -> PathBuf {
    sibling_path(active, ".restore-pending")
}

fn rollback_path(active: &Path) -> PathBuf {
    sibling_path(active, ".restore-rollback")
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn companion_path(path: &Path, suffix: &str) -> PathBuf {
    sibling_path(path, suffix)
}

fn rename_database_set(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination).map_err(StorageError::FileOperation)?;
    for suffix in ["-wal", "-shm"] {
        let source_companion = companion_path(source, suffix);
        if source_companion.exists() {
            fs::rename(&source_companion, companion_path(destination, suffix))
                .map_err(StorageError::FileOperation)?;
        }
    }
    Ok(())
}

fn remove_database_set(path: &Path) -> Result<(), StorageError> {
    remove_file_if_present(path)?;
    remove_file_if_present(&companion_path(path, "-wal"))?;
    remove_file_if_present(&companion_path(path, "-shm"))
}

fn remove_file_if_present(path: &Path) -> Result<(), StorageError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StorageError::FileOperation(error)),
    }
}

#[cfg(test)]
#[path = "tests/data_protection.rs"]
mod tests;

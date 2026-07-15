use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use thiserror::Error;
use wyrmgrid_storage::{
    DatabaseKey, PORTABLE_BACKUP_FORMAT_VERSION, PortableBackupRecord, PortableRestoreRecord,
    StorageError, Store,
};

const MINIMUM_BACKUP_PASSWORD_CHARACTERS: usize = 12;
const MAXIMUM_BACKUP_PASSWORD_BYTES: usize = 1_024;

pub trait DataProtectionRepository: Send + Sync + 'static {
    fn persistent(&self) -> bool;
    fn pending_restore(&self) -> Result<bool, DataProtectionError>;
    fn export_backup(
        &self,
        destination: &Path,
        password: &str,
        created_at: &str,
        application_version: &str,
    ) -> Result<PortableBackupRecord, DataProtectionError>;
    fn prepare_restore(
        &self,
        source: &Path,
        password: &str,
        device_key: &DatabaseKey,
    ) -> Result<PortableRestoreRecord, DataProtectionError>;
}

impl DataProtectionRepository for Store {
    fn persistent(&self) -> bool {
        self.is_persistent()
    }

    fn pending_restore(&self) -> Result<bool, DataProtectionError> {
        self.portable_backup_pending_restore()
            .map_err(DataProtectionError::from)
    }

    fn export_backup(
        &self,
        destination: &Path,
        password: &str,
        created_at: &str,
        application_version: &str,
    ) -> Result<PortableBackupRecord, DataProtectionError> {
        self.export_portable_backup(destination, password, created_at, application_version)
            .map_err(DataProtectionError::from)
    }

    fn prepare_restore(
        &self,
        source: &Path,
        password: &str,
        device_key: &DatabaseKey,
    ) -> Result<PortableRestoreRecord, DataProtectionError> {
        self.prepare_portable_restore(source, password, device_key)
            .map_err(DataProtectionError::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataProtectionStatus {
    pub database_encrypted: bool,
    pub device_key_protected: bool,
    pub portable_backup_format_version: u32,
    pub pending_restore: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortableBackupView {
    pub format_version: u32,
    pub schema_version: i64,
    pub created_at: String,
    pub application_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortableRestoreView {
    pub format_version: u32,
    pub schema_version: i64,
    pub backup_created_at: String,
    pub backup_application_version: String,
    pub restart_required: bool,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DataProtectionError {
    #[error("Use at least 12 characters for the portable-backup password.")]
    PasswordTooShort,
    #[error("The portable-backup password is too long.")]
    PasswordTooLong,
    #[error("The two portable-backup passwords do not match.")]
    PasswordConfirmationMismatch,
    #[error("Confirm that restoring will replace current local data after WyrmGrid restarts.")]
    RestoreConfirmationRequired,
    #[error("Choose a new filename; WyrmGrid will not overwrite an existing backup.")]
    DestinationExists,
    #[error(
        "That file is not a valid WyrmGrid portable backup, is damaged, or the password is wrong."
    )]
    InvalidBackup,
    #[error("Choose a portable backup rather than WyrmGrid's active database.")]
    SourceIsActiveDatabase,
    #[error("Portable backup and restore require persistent encrypted storage.")]
    PersistentStorageRequired,
    #[error("WyrmGrid could not complete the encrypted backup operation.")]
    StorageUnavailable,
}

impl From<StorageError> for DataProtectionError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::BackupDestinationExists => Self::DestinationExists,
            StorageError::InvalidPortableBackup => Self::InvalidBackup,
            StorageError::RestoreSourceIsActiveDatabase => Self::SourceIsActiveDatabase,
            StorageError::PersistentStorageRequired => Self::PersistentStorageRequired,
            _ => Self::StorageUnavailable,
        }
    }
}

#[derive(Clone)]
pub struct DataProtectionService<R> {
    repository: R,
}

impl<R: DataProtectionRepository> DataProtectionService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<DataProtectionStatus, DataProtectionError> {
        if !self.repository.persistent() {
            return Err(DataProtectionError::PersistentStorageRequired);
        }
        Ok(DataProtectionStatus {
            database_encrypted: true,
            device_key_protected: true,
            portable_backup_format_version: PORTABLE_BACKUP_FORMAT_VERSION,
            pending_restore: self.repository.pending_restore()?,
        })
    }

    pub fn create_portable_backup(
        &self,
        destination: &Path,
        password: &str,
        password_confirmation: &str,
    ) -> Result<PortableBackupView, DataProtectionError> {
        validate_password(password)?;
        if password != password_confirmation {
            return Err(DataProtectionError::PasswordConfirmationMismatch);
        }
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        self.repository
            .export_backup(
                destination,
                password,
                &created_at,
                env!("CARGO_PKG_VERSION"),
            )
            .map(portable_backup_view)
    }

    pub fn prepare_portable_restore(
        &self,
        source: &Path,
        password: &str,
        replacement_confirmed: bool,
        device_key: &DatabaseKey,
    ) -> Result<PortableRestoreView, DataProtectionError> {
        validate_password(password)?;
        if !replacement_confirmed {
            return Err(DataProtectionError::RestoreConfirmationRequired);
        }
        self.repository
            .prepare_restore(source, password, device_key)
            .map(portable_restore_view)
    }
}

fn validate_password(password: &str) -> Result<(), DataProtectionError> {
    if password.chars().count() < MINIMUM_BACKUP_PASSWORD_CHARACTERS {
        return Err(DataProtectionError::PasswordTooShort);
    }
    if password.len() > MAXIMUM_BACKUP_PASSWORD_BYTES {
        return Err(DataProtectionError::PasswordTooLong);
    }
    Ok(())
}

fn portable_backup_view(record: PortableBackupRecord) -> PortableBackupView {
    PortableBackupView {
        format_version: record.format_version,
        schema_version: record.schema_version,
        created_at: record.created_at,
        application_version: record.application_version,
    }
}

fn portable_restore_view(record: PortableRestoreRecord) -> PortableRestoreView {
    PortableRestoreView {
        format_version: record.format_version,
        schema_version: record.schema_version,
        backup_created_at: record.created_at,
        backup_application_version: record.application_version,
        restart_required: record.restart_required,
    }
}

#[cfg(test)]
#[path = "tests/data_protection.rs"]
mod tests;

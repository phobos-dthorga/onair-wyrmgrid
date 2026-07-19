use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::*;

#[derive(Default)]
struct MemoryDataProtectionRepository {
    persistent: bool,
    pending: bool,
    exported: Mutex<Option<PathBuf>>,
    restored: Mutex<Option<PathBuf>>,
    reset_prepared: Mutex<bool>,
}

impl DataProtectionRepository for MemoryDataProtectionRepository {
    fn persistent(&self) -> bool {
        self.persistent
    }

    fn pending_restore(&self) -> Result<bool, DataProtectionError> {
        Ok(self.pending)
    }

    fn export_backup(
        &self,
        destination: &Path,
        _password: &str,
        created_at: &str,
        application_version: &str,
    ) -> Result<PortableBackupRecord, DataProtectionError> {
        *self.exported.lock().unwrap() = Some(destination.to_path_buf());
        Ok(PortableBackupRecord {
            format_version: 1,
            schema_version: 9,
            created_at: created_at.into(),
            application_version: application_version.into(),
        })
    }

    fn prepare_restore(
        &self,
        source: &Path,
        _password: &str,
        _device_key: &DatabaseKey,
    ) -> Result<PortableRestoreRecord, DataProtectionError> {
        *self.restored.lock().unwrap() = Some(source.to_path_buf());
        Ok(PortableRestoreRecord {
            format_version: 1,
            schema_version: 9,
            created_at: "2026-07-15T00:00:00Z".into(),
            application_version: "0.1.0".into(),
            restart_required: true,
        })
    }

    fn prepare_local_data_reset(&self) -> Result<(), DataProtectionError> {
        *self.reset_prepared.lock().unwrap() = true;
        Ok(())
    }
}

#[test]
fn status_reports_encrypted_device_bound_storage() {
    let service = DataProtectionService::new(MemoryDataProtectionRepository {
        persistent: true,
        pending: true,
        ..Default::default()
    });
    assert_eq!(
        service.status().unwrap(),
        DataProtectionStatus {
            database_encrypted: true,
            device_key_protected: true,
            portable_backup_format_version: 1,
            pending_restore: true,
            local_data_reset_confirmation: LOCAL_DATA_RESET_CONFIRMATION,
        }
    );
}

#[test]
fn backup_requires_a_bounded_matching_password() {
    let service = DataProtectionService::new(MemoryDataProtectionRepository {
        persistent: true,
        ..Default::default()
    });
    let path = Path::new("journey.wyrmgrid-backup");
    assert_eq!(
        service.create_portable_backup(path, "too short", "too short"),
        Err(DataProtectionError::PasswordTooShort)
    );
    assert!(
        service
            .create_portable_backup(path, "123456789012", "123456789012")
            .is_ok()
    );
    assert_eq!(
        service.create_portable_backup(
            path,
            "this password is long enough",
            "a different password entirely"
        ),
        Err(DataProtectionError::PasswordConfirmationMismatch)
    );
    let oversized = "x".repeat(1_025);
    assert_eq!(
        service.create_portable_backup(path, &oversized, &oversized),
        Err(DataProtectionError::PasswordTooLong)
    );
}

#[test]
fn backup_and_restore_delegate_only_after_policy_checks() {
    let service = DataProtectionService::new(MemoryDataProtectionRepository {
        persistent: true,
        ..Default::default()
    });
    let password = "this password is long enough";
    let backup = service
        .create_portable_backup(Path::new("journey.wyrmgrid-backup"), password, password)
        .unwrap();
    assert_eq!(backup.format_version, 1);

    let key = DatabaseKey::from_bytes([31; 32]);
    assert_eq!(
        service.prepare_portable_restore(
            Path::new("journey.wyrmgrid-backup"),
            password,
            false,
            &key
        ),
        Err(DataProtectionError::RestoreConfirmationRequired)
    );
    assert!(
        service
            .prepare_portable_restore(Path::new("journey.wyrmgrid-backup"), password, true, &key)
            .unwrap()
            .restart_required
    );
}

#[test]
fn in_memory_fallback_cannot_claim_database_protection() {
    let service = DataProtectionService::new(MemoryDataProtectionRepository::default());
    assert_eq!(
        service.status(),
        Err(DataProtectionError::PersistentStorageRequired)
    );
}

#[test]
fn local_data_reset_requires_exact_confirmation_and_persistent_storage() {
    let repository = MemoryDataProtectionRepository {
        persistent: true,
        ..Default::default()
    };
    let service = DataProtectionService::new(repository);

    for confirmation in ["", "erase wyrmgrid data", "ERASE WYRMGRID DATA "] {
        assert_eq!(
            service.prepare_local_data_reset(confirmation),
            Err(DataProtectionError::LocalDataResetConfirmationRequired)
        );
    }
    assert!(
        service
            .prepare_local_data_reset(LOCAL_DATA_RESET_CONFIRMATION)
            .unwrap()
            .restart_required
    );
    assert!(*service.repository.reset_prepared.lock().unwrap());

    let in_memory = DataProtectionService::new(MemoryDataProtectionRepository::default());
    assert_eq!(
        in_memory.prepare_local_data_reset(LOCAL_DATA_RESET_CONFIRMATION),
        Err(DataProtectionError::PersistentStorageRequired)
    );
    assert!(!*in_memory.repository.reset_prepared.lock().unwrap());
}

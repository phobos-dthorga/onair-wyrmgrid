use std::sync::Mutex;

use super::*;

#[derive(Default)]
struct MemorySecretStore {
    value: Mutex<Option<Vec<u8>>>,
    fail_reads: bool,
    fail_writes: bool,
}

impl SecretStore for MemorySecretStore {
    fn read(&self) -> Result<Option<Vec<u8>>, DeviceKeyError> {
        if self.fail_reads {
            return Err(DeviceKeyError::CredentialStoreUnavailable);
        }
        Ok(self.value.lock().unwrap().clone())
    }

    fn write(&self, secret: &[u8]) -> Result<(), DeviceKeyError> {
        if self.fail_writes {
            return Err(DeviceKeyError::CredentialStoreUnavailable);
        }
        *self.value.lock().unwrap() = Some(secret.to_vec());
        Ok(())
    }
}

#[test]
fn creates_and_reuses_a_32_byte_device_key_for_a_new_database() {
    let store = MemorySecretStore::default();
    let first = load_or_create(&store, false).expect("new key should be created");
    let second = load_or_create(&store, true).expect("stored key should be reused");
    assert_eq!(first.expose(), second.expose());
    assert_ne!(first.expose(), &[0; 32]);
}

#[test]
fn existing_database_without_a_key_fails_closed() {
    let store = MemorySecretStore::default();
    assert!(matches!(
        load_or_create(&store, true),
        Err(DeviceKeyError::MissingForExistingDatabase)
    ));
}

#[test]
fn invalid_and_unavailable_keyring_entries_fail_closed() {
    let invalid = MemorySecretStore {
        value: Mutex::new(Some(vec![7; 31])),
        ..Default::default()
    };
    assert!(matches!(
        load_existing(&invalid),
        Err(DeviceKeyError::InvalidStoredKey)
    ));
    let unavailable = MemorySecretStore {
        fail_reads: true,
        ..Default::default()
    };
    assert!(matches!(
        load_existing(&unavailable),
        Err(DeviceKeyError::CredentialStoreUnavailable)
    ));
}

#[test]
fn failed_keyring_write_never_returns_an_unstored_key() {
    let store = MemorySecretStore {
        fail_writes: true,
        ..Default::default()
    };
    assert!(matches!(
        load_or_create(&store, false),
        Err(DeviceKeyError::CredentialStoreUnavailable)
    ));
}

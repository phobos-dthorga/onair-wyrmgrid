use keyring::{Entry, Error as KeyringError};
use thiserror::Error;
use zeroize::Zeroizing;

use wyrmgrid_storage::DatabaseKey;

const DATABASE_KEY_SERVICE: &str = "io.github.phobosdthorga.onairwyrmgrid";
const DATABASE_KEY_ACCOUNT: &str = "wyrmgrid-database-key-v1";
const DATABASE_KEY_BYTES: usize = 32;

#[derive(Debug, Error)]
pub enum DeviceKeyError {
    #[error("The operating-system credential store is unavailable.")]
    CredentialStoreUnavailable,
    #[error("The encrypted database exists, but its device key is missing.")]
    MissingForExistingDatabase,
    #[error("The stored database key is invalid.")]
    InvalidStoredKey,
    #[error("The operating system could not generate secure random data.")]
    RandomnessUnavailable,
}

#[derive(Clone, Default)]
pub struct DeviceKeyStore;

impl DeviceKeyStore {
    pub fn load_or_create(
        &self,
        encrypted_database_exists: bool,
    ) -> Result<DatabaseKey, DeviceKeyError> {
        load_or_create(&PlatformSecretStore, encrypted_database_exists)
    }

    pub fn load_existing(&self) -> Result<DatabaseKey, DeviceKeyError> {
        load_existing(&PlatformSecretStore)
    }
}

trait SecretStore {
    fn read(&self) -> Result<Option<Vec<u8>>, DeviceKeyError>;
    fn write(&self, secret: &[u8]) -> Result<(), DeviceKeyError>;
}

struct PlatformSecretStore;

impl SecretStore for PlatformSecretStore {
    fn read(&self) -> Result<Option<Vec<u8>>, DeviceKeyError> {
        let entry = platform_entry()?;
        match entry.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(DeviceKeyError::CredentialStoreUnavailable),
        }
    }

    fn write(&self, secret: &[u8]) -> Result<(), DeviceKeyError> {
        platform_entry()?
            .set_secret(secret)
            .map_err(|_| DeviceKeyError::CredentialStoreUnavailable)
    }
}

fn platform_entry() -> Result<Entry, DeviceKeyError> {
    Entry::new(DATABASE_KEY_SERVICE, DATABASE_KEY_ACCOUNT)
        .map_err(|_| DeviceKeyError::CredentialStoreUnavailable)
}

fn load_or_create<S: SecretStore>(
    store: &S,
    encrypted_database_exists: bool,
) -> Result<DatabaseKey, DeviceKeyError> {
    match store.read()? {
        Some(secret) => database_key(secret),
        None if encrypted_database_exists => Err(DeviceKeyError::MissingForExistingDatabase),
        None => {
            let mut bytes = Zeroizing::new([0_u8; DATABASE_KEY_BYTES]);
            getrandom::fill(bytes.as_mut()).map_err(|_| DeviceKeyError::RandomnessUnavailable)?;
            store.write(bytes.as_ref())?;
            DatabaseKey::try_from_slice(bytes.as_ref())
                .map_err(|_| DeviceKeyError::InvalidStoredKey)
        }
    }
}

fn load_existing<S: SecretStore>(store: &S) -> Result<DatabaseKey, DeviceKeyError> {
    store
        .read()?
        .ok_or(DeviceKeyError::MissingForExistingDatabase)
        .and_then(database_key)
}

fn database_key(secret: Vec<u8>) -> Result<DatabaseKey, DeviceKeyError> {
    let secret = Zeroizing::new(secret);
    DatabaseKey::try_from_slice(secret.as_slice()).map_err(|_| DeviceKeyError::InvalidStoredKey)
}

#[cfg(test)]
#[path = "tests/database_key.rs"]
mod tests;

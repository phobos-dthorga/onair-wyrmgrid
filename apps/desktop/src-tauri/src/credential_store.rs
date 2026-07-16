use keyring::{Entry, Error as KeyringError};
use secrecy::{ExposeSecret, SecretString};

use wyrmgrid_application::{AccountSettingsError, OnAirSecretStore};

const CREDENTIAL_SERVICE: &str = "io.github.phobosdthorga.onairwyrmgrid";
const ONAIR_API_KEY_ACCOUNT: &str = "onair-api-key-v1";

#[derive(Clone, Default)]
pub struct PlatformOnAirSecretStore;

impl OnAirSecretStore for PlatformOnAirSecretStore {
    fn load(&self) -> Result<Option<SecretString>, AccountSettingsError> {
        load(&PlatformCredentialBackend)
    }

    fn save(&self, secret: &SecretString) -> Result<(), AccountSettingsError> {
        save(&PlatformCredentialBackend, secret)
    }

    fn delete(&self) -> Result<(), AccountSettingsError> {
        delete(&PlatformCredentialBackend)
    }
}

trait CredentialBackend {
    fn read(&self) -> Result<Option<Vec<u8>>, AccountSettingsError>;
    fn write(&self, secret: &[u8]) -> Result<(), AccountSettingsError>;
    fn remove(&self) -> Result<(), AccountSettingsError>;
}

struct PlatformCredentialBackend;

impl CredentialBackend for PlatformCredentialBackend {
    fn read(&self) -> Result<Option<Vec<u8>>, AccountSettingsError> {
        match entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(AccountSettingsError::CredentialStoreUnavailable),
        }
    }

    fn write(&self, secret: &[u8]) -> Result<(), AccountSettingsError> {
        entry()?
            .set_secret(secret)
            .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)
    }

    fn remove(&self) -> Result<(), AccountSettingsError> {
        match entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(_) => Err(AccountSettingsError::CredentialStoreUnavailable),
        }
    }
}

fn load<B: CredentialBackend>(backend: &B) -> Result<Option<SecretString>, AccountSettingsError> {
    backend
        .read()?
        .map(|secret| {
            String::from_utf8(secret)
                .map(SecretString::from)
                .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)
        })
        .transpose()
}

fn save<B: CredentialBackend>(
    backend: &B,
    secret: &SecretString,
) -> Result<(), AccountSettingsError> {
    backend.write(secret.expose_secret().as_bytes())
}

fn delete<B: CredentialBackend>(backend: &B) -> Result<(), AccountSettingsError> {
    backend.remove()
}

fn entry() -> Result<Entry, AccountSettingsError> {
    Entry::new(CREDENTIAL_SERVICE, ONAIR_API_KEY_ACCOUNT)
        .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)
}

#[cfg(test)]
#[path = "tests/credential_store.rs"]
mod tests;

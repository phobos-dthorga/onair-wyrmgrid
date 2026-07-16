use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct MemoryCredentialBackend {
    value: Mutex<Option<Vec<u8>>>,
    unavailable: bool,
}

impl CredentialBackend for MemoryCredentialBackend {
    fn read(&self) -> Result<Option<Vec<u8>>, AccountSettingsError> {
        if self.unavailable {
            return Err(AccountSettingsError::CredentialStoreUnavailable);
        }
        Ok(self.value.lock().unwrap().clone())
    }

    fn write(&self, secret: &[u8]) -> Result<(), AccountSettingsError> {
        if self.unavailable {
            return Err(AccountSettingsError::CredentialStoreUnavailable);
        }
        *self.value.lock().unwrap() = Some(secret.to_vec());
        Ok(())
    }

    fn remove(&self) -> Result<(), AccountSettingsError> {
        if self.unavailable {
            return Err(AccountSettingsError::CredentialStoreUnavailable);
        }
        *self.value.lock().unwrap() = None;
        Ok(())
    }
}

#[test]
fn saves_loads_and_forgets_the_onair_secret() {
    let backend = MemoryCredentialBackend::default();
    let secret = SecretString::from("canary-api-key".to_owned());

    save(&backend, &secret).unwrap();
    assert_eq!(
        load(&backend)
            .unwrap()
            .expect("secret should exist")
            .expose_secret(),
        "canary-api-key"
    );
    delete(&backend).unwrap();
    assert!(load(&backend).unwrap().is_none());
}

#[test]
fn unavailable_store_fails_closed() {
    let backend = MemoryCredentialBackend {
        unavailable: true,
        ..Default::default()
    };
    let secret = SecretString::from("canary-api-key".to_owned());

    assert!(matches!(
        load(&backend),
        Err(AccountSettingsError::CredentialStoreUnavailable)
    ));
    assert!(matches!(
        save(&backend, &secret),
        Err(AccountSettingsError::CredentialStoreUnavailable)
    ));
    assert!(matches!(
        delete(&backend),
        Err(AccountSettingsError::CredentialStoreUnavailable)
    ));
}

#[test]
fn invalid_stored_secret_fails_closed() {
    let backend = MemoryCredentialBackend {
        value: Mutex::new(Some(vec![0xff, 0xfe])),
        ..Default::default()
    };

    assert!(matches!(
        load(&backend),
        Err(AccountSettingsError::CredentialStoreUnavailable)
    ));
}

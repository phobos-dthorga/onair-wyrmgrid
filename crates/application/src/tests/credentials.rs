use super::*;
use secrecy::{ExposeSecret, SecretString};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
struct MemoryAccountPreferences {
    onair: Mutex<Option<RememberedOnAirAccount>>,
    simbrief: Mutex<Option<SimBriefAccountPreference>>,
    fail_onair_save: AtomicBool,
}

impl AccountPreferencesRepository for MemoryAccountPreferences {
    fn load_onair_account(&self) -> Result<Option<RememberedOnAirAccount>, AccountSettingsError> {
        self.onair
            .lock()
            .map(|value| value.clone())
            .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn save_onair_account(
        &self,
        account: &RememberedOnAirAccount,
    ) -> Result<(), AccountSettingsError> {
        if self.fail_onair_save.load(Ordering::SeqCst) {
            return Err(AccountSettingsError::StorageUnavailable);
        }
        *self
            .onair
            .lock()
            .map_err(|_| AccountSettingsError::StorageUnavailable)? = Some(account.clone());
        Ok(())
    }

    fn delete_onair_account(&self) -> Result<(), AccountSettingsError> {
        *self
            .onair
            .lock()
            .map_err(|_| AccountSettingsError::StorageUnavailable)? = None;
        Ok(())
    }

    fn load_simbrief_account(
        &self,
    ) -> Result<Option<SimBriefAccountPreference>, AccountSettingsError> {
        self.simbrief
            .lock()
            .map(|value| value.clone())
            .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn save_simbrief_account(
        &self,
        account: &SimBriefAccountPreference,
    ) -> Result<(), AccountSettingsError> {
        *self
            .simbrief
            .lock()
            .map_err(|_| AccountSettingsError::StorageUnavailable)? = Some(account.clone());
        Ok(())
    }

    fn delete_simbrief_account(&self) -> Result<(), AccountSettingsError> {
        *self
            .simbrief
            .lock()
            .map_err(|_| AccountSettingsError::StorageUnavailable)? = None;
        Ok(())
    }
}

#[derive(Default)]
struct MemorySecretStore {
    secret: Mutex<Option<String>>,
}

impl OnAirSecretStore for MemorySecretStore {
    fn load(&self) -> Result<Option<SecretString>, AccountSettingsError> {
        self.secret
            .lock()
            .map(|value| value.clone().map(SecretString::from))
            .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)
    }

    fn save(&self, secret: &SecretString) -> Result<(), AccountSettingsError> {
        *self
            .secret
            .lock()
            .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)? =
            Some(secret.expose_secret().to_owned());
        Ok(())
    }

    fn delete(&self) -> Result<(), AccountSettingsError> {
        *self
            .secret
            .lock()
            .map_err(|_| AccountSettingsError::CredentialStoreUnavailable)? = None;
        Ok(())
    }
}

fn service() -> AccountSettingsService<MemoryAccountPreferences, MemorySecretStore> {
    AccountSettingsService::new(
        MemoryAccountPreferences::default(),
        MemorySecretStore::default(),
        OnAirSession::default(),
    )
}

#[test]
fn simbrief_pilot_id_is_remembered_only_when_explicitly_selected() {
    let service = service();

    let remembered = service
        .remember_simbrief(SimBriefReferenceKind::PilotId, " 1234567 ", true)
        .unwrap();
    assert_eq!(
        remembered,
        Some(SimBriefAccountPreference {
            reference_kind: SimBriefReferenceKind::PilotId,
            reference: "1234567".to_owned(),
        })
    );
    assert_eq!(service.simbrief_status().unwrap(), remembered);

    assert_eq!(
        service
            .remember_simbrief(SimBriefReferenceKind::PilotId, "1234567", false)
            .unwrap(),
        None
    );
    assert_eq!(service.simbrief_status().unwrap(), None);
}

#[test]
fn invalid_simbrief_reference_is_not_persisted() {
    let service = service();

    for (kind, reference) in [
        (SimBriefReferenceKind::PilotId, " "),
        (SimBriefReferenceKind::PilotId, "dragon-seven"),
        (SimBriefReferenceKind::Username, "not valid!"),
    ] {
        assert_eq!(
            service.remember_simbrief(kind, reference, true),
            Err(AccountSettingsError::InvalidSimBriefReference)
        );
    }
    assert_eq!(service.simbrief_status().unwrap(), None);
}

#[test]
fn empty_onair_profile_does_not_probe_or_expose_the_secret_store() {
    let service = service();

    assert_eq!(
        service.onair_status().unwrap(),
        OnAirCredentialProfileStatus {
            remembered: false,
            company_id: None,
            connect_on_start: false,
            secret_available: false,
            credential_store_available: true,
        }
    );
}

#[test]
fn reports_and_forgets_remembered_onair_metadata_and_secret_together() {
    let service = service();
    *service.repository.onair.lock().unwrap() = Some(RememberedOnAirAccount {
        company_id: "75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned(),
        connect_on_start: true,
    });
    *service.secret_store.secret.lock().unwrap() = Some("canary-api-key".to_owned());

    assert_eq!(
        service.onair_status().unwrap(),
        OnAirCredentialProfileStatus {
            remembered: true,
            company_id: Some("75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned()),
            connect_on_start: true,
            secret_available: true,
            credential_store_available: true,
        }
    );

    assert_eq!(
        service.forget_onair().unwrap(),
        OnAirCredentialProfileStatus {
            remembered: false,
            company_id: None,
            connect_on_start: false,
            secret_available: false,
            credential_store_available: true,
        }
    );
    assert!(service.secret_store.secret.lock().unwrap().is_none());
}

#[test]
fn remembered_metadata_without_a_key_is_visible_but_not_connectable() {
    let service = service();
    *service.repository.onair.lock().unwrap() = Some(RememberedOnAirAccount {
        company_id: "75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned(),
        connect_on_start: false,
    });

    let status = service.onair_status().unwrap();
    assert!(status.remembered);
    assert!(!status.secret_available);
    assert!(status.credential_store_available);
}

#[tokio::test]
async fn automatic_connection_is_inert_without_an_explicit_saved_preference() {
    let service = service();

    assert_eq!(service.connect_on_start_if_enabled().await.unwrap(), None);
}

#[test]
fn metadata_failure_removes_the_newly_written_onair_secret() {
    let service = service();
    service
        .repository
        .fail_onair_save
        .store(true, Ordering::SeqCst);
    let account = RememberedOnAirAccount {
        company_id: "75a2c304-3f5c-49c8-974d-23c10ad14cc2".to_owned(),
        connect_on_start: false,
    };
    let secret = SecretString::from("canary-api-key".to_owned());

    assert_eq!(
        service.persist_onair_account(&account, &secret),
        Err(AccountSettingsError::StorageUnavailable)
    );
    assert!(service.secret_store.secret.lock().unwrap().is_none());
    assert!(service.repository.onair.lock().unwrap().is_none());
}

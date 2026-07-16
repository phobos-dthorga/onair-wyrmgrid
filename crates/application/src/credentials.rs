//! Optional provider-account persistence without exposing secrets to interface layers.

use secrecy::SecretString;
use serde::Serialize;
use thiserror::Error;
use wyrmgrid_simbrief_api::UserReference;
use wyrmgrid_storage::{OnAirAccountPreferencesRecord, SimBriefAccountPreferencesRecord, Store};
use zeroize::Zeroize;

use crate::{ConnectionError, ConnectionStatus, OnAirSession, SimBriefReferenceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RememberedOnAirAccount {
    pub company_id: String,
    pub connect_on_start: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnAirCredentialProfileStatus {
    pub remembered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    pub connect_on_start: bool,
    pub secret_available: bool,
    pub credential_store_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnAirConnectionResult {
    pub connection: ConnectionStatus,
    pub profile: OnAirCredentialProfileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimBriefAccountPreference {
    pub reference_kind: SimBriefReferenceKind,
    pub reference: String,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AccountSettingsError {
    #[error("WyrmGrid could not read or save its encrypted account settings.")]
    StorageUnavailable,
    #[error("The operating-system credential store is unavailable.")]
    CredentialStoreUnavailable,
    #[error(
        "The remembered OnAir API key is missing. Enter it again to replace the saved connection."
    )]
    RememberedSecretMissing,
    #[error("No remembered OnAir connection is available.")]
    RememberedAccountMissing,
    #[error("That SimBrief account reference is invalid.")]
    InvalidSimBriefReference,
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

pub trait AccountPreferencesRepository: Send + Sync + 'static {
    fn load_onair_account(&self) -> Result<Option<RememberedOnAirAccount>, AccountSettingsError>;
    fn save_onair_account(
        &self,
        account: &RememberedOnAirAccount,
    ) -> Result<(), AccountSettingsError>;
    fn delete_onair_account(&self) -> Result<(), AccountSettingsError>;
    fn load_simbrief_account(
        &self,
    ) -> Result<Option<SimBriefAccountPreference>, AccountSettingsError>;
    fn save_simbrief_account(
        &self,
        account: &SimBriefAccountPreference,
    ) -> Result<(), AccountSettingsError>;
    fn delete_simbrief_account(&self) -> Result<(), AccountSettingsError>;
}

impl AccountPreferencesRepository for Store {
    fn load_onair_account(&self) -> Result<Option<RememberedOnAirAccount>, AccountSettingsError> {
        self.load_onair_account_preferences_record()
            .map(|record| {
                record.map(|record| RememberedOnAirAccount {
                    company_id: record.company_id,
                    connect_on_start: record.connect_on_start,
                })
            })
            .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn save_onair_account(
        &self,
        account: &RememberedOnAirAccount,
    ) -> Result<(), AccountSettingsError> {
        self.save_onair_account_preferences_record(&OnAirAccountPreferencesRecord {
            company_id: account.company_id.clone(),
            connect_on_start: account.connect_on_start,
        })
        .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn delete_onair_account(&self) -> Result<(), AccountSettingsError> {
        self.delete_onair_account_preferences_record()
            .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn load_simbrief_account(
        &self,
    ) -> Result<Option<SimBriefAccountPreference>, AccountSettingsError> {
        self.load_simbrief_account_preferences_record()
            .map_err(|_| AccountSettingsError::StorageUnavailable)?
            .map(|record| {
                let reference_kind = parse_simbrief_kind(&record.reference_kind)?;
                Ok(SimBriefAccountPreference {
                    reference_kind,
                    reference: normalize_simbrief_reference(reference_kind, &record.reference)?,
                })
            })
            .transpose()
    }

    fn save_simbrief_account(
        &self,
        account: &SimBriefAccountPreference,
    ) -> Result<(), AccountSettingsError> {
        self.save_simbrief_account_preferences_record(&SimBriefAccountPreferencesRecord {
            reference_kind: simbrief_kind(account.reference_kind).to_owned(),
            reference: account.reference.clone(),
        })
        .map_err(|_| AccountSettingsError::StorageUnavailable)
    }

    fn delete_simbrief_account(&self) -> Result<(), AccountSettingsError> {
        self.delete_simbrief_account_preferences_record()
            .map_err(|_| AccountSettingsError::StorageUnavailable)
    }
}

pub trait OnAirSecretStore: Send + Sync + 'static {
    fn load(&self) -> Result<Option<SecretString>, AccountSettingsError>;
    fn save(&self, secret: &SecretString) -> Result<(), AccountSettingsError>;
    fn delete(&self) -> Result<(), AccountSettingsError>;
}

#[derive(Clone)]
pub struct AccountSettingsService<R, S> {
    repository: R,
    secret_store: S,
    onair: OnAirSession,
}

impl<R: AccountPreferencesRepository, S: OnAirSecretStore> AccountSettingsService<R, S> {
    pub fn new(repository: R, secret_store: S, onair: OnAirSession) -> Self {
        Self {
            repository,
            secret_store,
            onair,
        }
    }

    pub fn onair_status(&self) -> Result<OnAirCredentialProfileStatus, AccountSettingsError> {
        let Some(account) = self.repository.load_onair_account()? else {
            return Ok(OnAirCredentialProfileStatus {
                remembered: false,
                company_id: None,
                connect_on_start: false,
                secret_available: false,
                credential_store_available: true,
            });
        };

        let (secret_available, credential_store_available) = match self.secret_store.load() {
            Ok(secret) => (secret.is_some(), true),
            Err(AccountSettingsError::CredentialStoreUnavailable) => (false, false),
            Err(error) => return Err(error),
        };
        Ok(OnAirCredentialProfileStatus {
            remembered: true,
            company_id: Some(account.company_id),
            connect_on_start: account.connect_on_start,
            secret_available,
            credential_store_available,
        })
    }

    pub async fn connect(
        &self,
        company_id: String,
        mut api_key: String,
        remember: bool,
        connect_on_start: bool,
    ) -> Result<OnAirConnectionResult, AccountSettingsError> {
        let trimmed_secret = api_key.trim().to_owned();
        api_key.zeroize();
        let secret = SecretString::from(trimmed_secret);
        let normalized_company_id = company_id.trim().to_owned();
        let connection = self
            .onair
            .connect_secret(normalized_company_id.clone(), &secret)
            .await?;

        if remember {
            let account = RememberedOnAirAccount {
                company_id: normalized_company_id,
                connect_on_start,
            };
            if let Err(error) = self.persist_onair_account(&account, &secret) {
                let _ = self.onair.disconnect();
                return Err(error);
            }
        } else {
            match self.repository.load_onair_account() {
                Ok(Some(_)) => {
                    if let Err(error) = self.forget_onair() {
                        let _ = self.onair.disconnect();
                        return Err(error);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = self.onair.disconnect();
                    return Err(error);
                }
            }
        }

        Ok(OnAirConnectionResult {
            connection,
            profile: self.onair_status()?,
        })
    }

    fn persist_onair_account(
        &self,
        account: &RememberedOnAirAccount,
        secret: &SecretString,
    ) -> Result<(), AccountSettingsError> {
        self.secret_store.save(secret)?;
        if let Err(error) = self.repository.save_onair_account(account) {
            let _ = self.secret_store.delete();
            return Err(error);
        }
        Ok(())
    }

    pub async fn connect_remembered(&self) -> Result<OnAirConnectionResult, AccountSettingsError> {
        let account = self
            .repository
            .load_onair_account()?
            .ok_or(AccountSettingsError::RememberedAccountMissing)?;
        let secret = self
            .secret_store
            .load()?
            .ok_or(AccountSettingsError::RememberedSecretMissing)?;
        let connection = self
            .onair
            .connect_secret(account.company_id, &secret)
            .await?;
        Ok(OnAirConnectionResult {
            connection,
            profile: self.onair_status()?,
        })
    }

    pub async fn connect_on_start_if_enabled(
        &self,
    ) -> Result<Option<OnAirConnectionResult>, AccountSettingsError> {
        if !self
            .repository
            .load_onair_account()?
            .is_some_and(|account| account.connect_on_start)
        {
            return Ok(None);
        }
        self.connect_remembered().await.map(Some)
    }

    pub fn forget_onair(&self) -> Result<OnAirCredentialProfileStatus, AccountSettingsError> {
        self.secret_store.delete()?;
        self.repository.delete_onair_account()?;
        self.onair_status()
    }

    pub fn simbrief_status(
        &self,
    ) -> Result<Option<SimBriefAccountPreference>, AccountSettingsError> {
        self.repository.load_simbrief_account()
    }

    pub fn remember_simbrief(
        &self,
        reference_kind: SimBriefReferenceKind,
        reference: &str,
        remember: bool,
    ) -> Result<Option<SimBriefAccountPreference>, AccountSettingsError> {
        if !remember {
            self.repository.delete_simbrief_account()?;
            return Ok(None);
        }
        let reference = normalize_simbrief_reference(reference_kind, reference)?;
        let account = SimBriefAccountPreference {
            reference_kind,
            reference,
        };
        self.repository.save_simbrief_account(&account)?;
        Ok(Some(account))
    }
}

fn simbrief_kind(kind: SimBriefReferenceKind) -> &'static str {
    match kind {
        SimBriefReferenceKind::PilotId => "pilot_id",
        SimBriefReferenceKind::Username => "username",
    }
}

fn parse_simbrief_kind(value: &str) -> Result<SimBriefReferenceKind, AccountSettingsError> {
    match value {
        "pilot_id" => Ok(SimBriefReferenceKind::PilotId),
        "username" => Ok(SimBriefReferenceKind::Username),
        _ => Err(AccountSettingsError::InvalidSimBriefReference),
    }
}

fn normalize_simbrief_reference(
    kind: SimBriefReferenceKind,
    value: &str,
) -> Result<String, AccountSettingsError> {
    UserReference::parse(kind.into(), value)
        .map_err(|_| AccountSettingsError::InvalidSimBriefReference)?;
    Ok(value.trim().to_owned())
}

#[cfg(test)]
#[path = "tests/credentials.rs"]
mod tests;

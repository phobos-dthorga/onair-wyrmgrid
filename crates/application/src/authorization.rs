use std::collections::BTreeSet;

use serde::Serialize;
use thiserror::Error;
use wyrmgrid_storage::Store;

pub const TERMS_VERSION: &str = "2026-07-14";
pub const PRIVACY_NOTICE_VERSION: &str = "2026-07-15";

const MAX_SUBJECT_ID_BYTES: usize = 256;
const MAX_SCOPE_REVISION_BYTES: usize = 1_024;
const MAX_CAPABILITY_BYTES: usize = 128;
const MAX_CAPABILITIES_PER_SUBJECT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalPreferences {
    pub terms_version: String,
    pub privacy_notice_version: String,
    pub telemetry_enabled: bool,
    pub acknowledged_at: String,
}

pub trait LegalPreferencesRepository: Send + Sync + 'static {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError>;

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError>;
}

impl LegalPreferencesRepository for Store {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
        self.load_legal_preferences_record()
            .map(|preferences| {
                preferences.map(|preferences| PersistedLegalPreferences {
                    terms_version: preferences.terms_version,
                    privacy_notice_version: preferences.privacy_notice_version,
                    telemetry_enabled: preferences.telemetry_enabled,
                    acknowledged_at: preferences.acknowledged_at,
                })
            })
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError> {
        self.save_legal_preferences_record(terms_version, privacy_notice_version, telemetry_enabled)
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegalStatus {
    pub terms_version: &'static str,
    pub privacy_notice_version: &'static str,
    pub acknowledged: bool,
    pub telemetry_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<String>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LegalSettingsError {
    #[error("WyrmGrid could not read or save its local privacy preferences.")]
    StorageUnavailable,
    #[error("Review the current Terms and Privacy Notice before changing this preference.")]
    AcknowledgementRequired,
}

pub struct LegalSettingsService<R> {
    repository: R,
}

impl<R: LegalPreferencesRepository> LegalSettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<LegalStatus, LegalSettingsError> {
        let stored = self.repository.load_legal_preferences()?;
        let acknowledged = stored.as_ref().is_some_and(|preferences| {
            preferences.terms_version == TERMS_VERSION
                && preferences.privacy_notice_version == PRIVACY_NOTICE_VERSION
        });
        let acknowledged_at = if acknowledged {
            stored
                .as_ref()
                .map(|preferences| preferences.acknowledged_at.clone())
        } else {
            None
        };

        Ok(LegalStatus {
            terms_version: TERMS_VERSION,
            privacy_notice_version: PRIVACY_NOTICE_VERSION,
            acknowledged,
            telemetry_enabled: acknowledged
                && stored
                    .as_ref()
                    .is_some_and(|preferences| preferences.telemetry_enabled),
            acknowledged_at,
        })
    }

    pub fn acknowledge(&self, telemetry_enabled: bool) -> Result<LegalStatus, LegalSettingsError> {
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }

    pub fn update_telemetry(
        &self,
        telemetry_enabled: bool,
    ) -> Result<LegalStatus, LegalSettingsError> {
        if !self.status()?.acknowledged {
            return Err(LegalSettingsError::AcknowledgementRequired);
        }
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorizationSubjectKind {
    Plugin,
}

impl AuthorizationSubjectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizationSubject {
    kind: AuthorizationSubjectKind,
    id: String,
}

impl AuthorizationSubject {
    pub(crate) fn plugin(id: impl Into<String>) -> Self {
        Self {
            kind: AuthorizationSubjectKind::Plugin,
            id: id.into(),
        }
    }
}

pub(crate) trait AuthorizationRepository: Send + Sync + 'static {
    fn list_grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
    ) -> Result<Vec<String>, AuthorizationError>;

    fn replace_grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        capabilities: &[String],
    ) -> Result<(), AuthorizationError>;
}

impl AuthorizationRepository for Store {
    fn list_grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
    ) -> Result<Vec<String>, AuthorizationError> {
        self.list_authorization_grant_records(subject.kind.as_str(), &subject.id, scope_revision)
            .map_err(|_| AuthorizationError::StorageUnavailable)
    }

    fn replace_grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        capabilities: &[String],
    ) -> Result<(), AuthorizationError> {
        self.replace_authorization_grant_records(
            subject.kind.as_str(),
            &subject.id,
            scope_revision,
            capabilities,
        )
        .map_err(|_| AuthorizationError::StorageUnavailable)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorizationError {
    #[error("The authorization subject is invalid.")]
    InvalidSubject,
    #[error("The authorization scope revision is invalid.")]
    InvalidScopeRevision,
    #[error("One or more authorization capabilities is invalid.")]
    InvalidCapability,
    #[error("A required capability has not been granted.")]
    CapabilityRequired,
    #[error("WyrmGrid could not read or save local authorization grants.")]
    StorageUnavailable,
}

#[derive(Clone)]
pub(crate) struct AuthorizationService<R> {
    repository: R,
}

impl<R: AuthorizationRepository> AuthorizationService<R> {
    pub(crate) fn new(repository: R) -> Self {
        Self { repository }
    }

    pub(crate) fn grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        requested: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        validate_request(subject, scope_revision, requested)?;
        self.repository
            .list_grants(subject, scope_revision)
            .map(|records| {
                records
                    .into_iter()
                    .filter(|capability| requested.contains(capability))
                    .collect()
            })
    }

    pub(crate) fn approve(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        capabilities: &BTreeSet<String>,
    ) -> Result<(), AuthorizationError> {
        validate_request(subject, scope_revision, capabilities)?;
        self.repository.replace_grants(
            subject,
            scope_revision,
            &capabilities.iter().cloned().collect::<Vec<_>>(),
        )
    }

    pub(crate) fn revoke(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
    ) -> Result<(), AuthorizationError> {
        validate_subject(subject)?;
        validate_scope_revision(scope_revision)?;
        self.repository.replace_grants(subject, scope_revision, &[])
    }

    pub(crate) fn require_all(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        requested: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let granted = self.grants(subject, scope_revision, requested)?;
        if requested.iter().any(|item| !granted.contains(item)) {
            return Err(AuthorizationError::CapabilityRequired);
        }
        Ok(granted)
    }
}

fn validate_request(
    subject: &AuthorizationSubject,
    scope_revision: &str,
    capabilities: &BTreeSet<String>,
) -> Result<(), AuthorizationError> {
    validate_subject(subject)?;
    validate_scope_revision(scope_revision)?;
    if capabilities.len() > MAX_CAPABILITIES_PER_SUBJECT
        || capabilities.iter().any(|capability| {
            capability.is_empty()
                || capability.len() > MAX_CAPABILITY_BYTES
                || !capability
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(AuthorizationError::InvalidCapability);
    }
    Ok(())
}

fn validate_subject(subject: &AuthorizationSubject) -> Result<(), AuthorizationError> {
    if subject.id.is_empty()
        || subject.id.len() > MAX_SUBJECT_ID_BYTES
        || !subject.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return Err(AuthorizationError::InvalidSubject);
    }
    Ok(())
}

fn validate_scope_revision(scope_revision: &str) -> Result<(), AuthorizationError> {
    if scope_revision.is_empty()
        || scope_revision.len() > MAX_SCOPE_REVISION_BYTES
        || !scope_revision
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'\'' && byte != b'"')
    {
        return Err(AuthorizationError::InvalidScopeRevision);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/authorization.rs"]
mod tests;

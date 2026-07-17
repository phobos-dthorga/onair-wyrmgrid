use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wyrmgrid_storage::{AuthorizationDecisionRecord, AuthorizationGrantRecord, Store};

pub const TERMS_VERSION: &str = "2026-07-15";
pub const PRIVACY_NOTICE_VERSION: &str = "2026-07-17.2";

const MAX_SUBJECT_ID_BYTES: usize = 256;
const MAX_SCOPE_REVISION_BYTES: usize = 1_024;
const MAX_CAPABILITY_BYTES: usize = 128;
const MAX_CAPABILITIES_PER_SUBJECT: usize = 128;
const MAX_ACTIVE_GRANT_RECORDS: usize = 4_096;
const MAX_VISIBLE_DECISIONS: usize = 100;
pub const AUTHORIZATION_DECISION_RETENTION_LIMIT: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationGrantLifetime {
    Once,
    Session,
    Standing,
}

#[derive(Clone, Default)]
pub struct AuthorizationRuntime {
    inner: Arc<Mutex<AuthorizationRuntimeState>>,
}

#[derive(Default)]
struct AuthorizationRuntimeState {
    grants: BTreeMap<RuntimeGrantKey, RuntimeGrant>,
    decisions: Vec<RuntimeDecision>,
    next_decision_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeGrantKey {
    subject_kind: String,
    subject_id: String,
    scope_revision: String,
}

#[derive(Debug, Clone)]
struct RuntimeGrant {
    capabilities: BTreeSet<String>,
    lifetime: AuthorizationGrantLifetime,
    granted_at: String,
}

#[derive(Debug, Clone)]
struct RuntimeDecision {
    id: i64,
    subject_kind: String,
    subject_id: String,
    scope_revision: String,
    decision: SecurityDecision,
    capability_count: u32,
    lifetime: Option<AuthorizationGrantLifetime>,
    decided_at: String,
}

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
        self.repository
            .load_legal_preferences()
            .map(legal_status_from_preferences)
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

fn legal_status_from_preferences(stored: Option<PersistedLegalPreferences>) -> LegalStatus {
    let acknowledged = stored.as_ref().is_some_and(|preferences| {
        preferences.terms_version == TERMS_VERSION
            && preferences.privacy_notice_version == PRIVACY_NOTICE_VERSION
    });
    let acknowledged_at = acknowledged
        .then(|| {
            stored
                .as_ref()
                .map(|preferences| preferences.acknowledged_at.clone())
        })
        .flatten();

    LegalStatus {
        terms_version: TERMS_VERSION,
        privacy_notice_version: PRIVACY_NOTICE_VERSION,
        acknowledged,
        telemetry_enabled: acknowledged
            && stored
                .as_ref()
                .is_some_and(|preferences| preferences.telemetry_enabled),
        acknowledged_at,
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
    runtime: AuthorizationRuntime,
}

impl<R: AuthorizationRepository> AuthorizationService<R> {
    pub(crate) fn with_runtime(repository: R, runtime: AuthorizationRuntime) -> Self {
        Self {
            repository,
            runtime,
        }
    }

    pub(crate) fn grants(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        requested: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        validate_request(subject, scope_revision, requested)?;
        let mut granted: BTreeSet<String> = self
            .repository
            .list_grants(subject, scope_revision)
            .map(|records| {
                records
                    .into_iter()
                    .filter(|capability| requested.contains(capability))
                    .collect()
            })?;
        if let Some(temporary) = self.runtime_grant(subject, scope_revision)? {
            granted.extend(
                temporary
                    .capabilities
                    .into_iter()
                    .filter(|capability| requested.contains(capability)),
            );
        }
        Ok(granted)
    }

    pub(crate) fn approve_with_lifetime(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        capabilities: &BTreeSet<String>,
        lifetime: AuthorizationGrantLifetime,
    ) -> Result<(), AuthorizationError> {
        validate_request(subject, scope_revision, capabilities)?;
        match lifetime {
            AuthorizationGrantLifetime::Standing => {
                self.repository.replace_grants(
                    subject,
                    scope_revision,
                    &capabilities.iter().cloned().collect::<Vec<_>>(),
                )?;
                self.clear_runtime_grant(subject, scope_revision, false)?;
            }
            AuthorizationGrantLifetime::Once | AuthorizationGrantLifetime::Session => {
                if !self
                    .repository
                    .list_grants(subject, scope_revision)?
                    .is_empty()
                {
                    self.repository
                        .replace_grants(subject, scope_revision, &[])?;
                }
                self.set_runtime_grant(subject, scope_revision, capabilities, lifetime)?;
            }
        }
        Ok(())
    }

    pub(crate) fn revoke(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
    ) -> Result<(), AuthorizationError> {
        validate_subject(subject)?;
        validate_scope_revision(scope_revision)?;
        if !self
            .repository
            .list_grants(subject, scope_revision)?
            .is_empty()
        {
            self.repository
                .replace_grants(subject, scope_revision, &[])?;
        }
        self.clear_runtime_grant(subject, scope_revision, true)
    }

    pub(crate) fn require_all(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        requested: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        validate_request(subject, scope_revision, requested)?;
        let standing = self
            .repository
            .list_grants(subject, scope_revision)?
            .into_iter()
            .filter(|capability| requested.contains(capability))
            .collect::<BTreeSet<_>>();
        let temporary = self.runtime_grant(subject, scope_revision)?;
        let mut granted = standing.clone();
        if let Some(temporary) = &temporary {
            granted.extend(
                temporary
                    .capabilities
                    .iter()
                    .filter(|capability| requested.contains(*capability))
                    .cloned(),
            );
        }
        if requested.iter().any(|item| !granted.contains(item)) {
            return Err(AuthorizationError::CapabilityRequired);
        }
        if temporary.as_ref().is_some_and(|grant| {
            grant.lifetime == AuthorizationGrantLifetime::Once
                && requested
                    .iter()
                    .any(|capability| !standing.contains(capability))
        }) {
            self.clear_runtime_grant(subject, scope_revision, false)?;
        }
        Ok(granted)
    }

    pub(crate) fn grant_lifetime(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        requested: &BTreeSet<String>,
    ) -> Result<Option<AuthorizationGrantLifetime>, AuthorizationError> {
        validate_request(subject, scope_revision, requested)?;
        let standing = self.repository.list_grants(subject, scope_revision)?;
        if requested.iter().all(|item| standing.contains(item)) {
            return Ok(Some(AuthorizationGrantLifetime::Standing));
        }
        Ok(self
            .runtime_grant(subject, scope_revision)?
            .filter(|grant| {
                requested
                    .iter()
                    .all(|item| grant.capabilities.contains(item))
            })
            .map(|grant| grant.lifetime))
    }

    fn runtime_grant(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
    ) -> Result<Option<RuntimeGrant>, AuthorizationError> {
        self.runtime
            .inner
            .lock()
            .map_err(|_| AuthorizationError::StorageUnavailable)
            .map(|state| {
                state
                    .grants
                    .get(&runtime_key(subject, scope_revision))
                    .cloned()
            })
    }

    fn set_runtime_grant(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        capabilities: &BTreeSet<String>,
        lifetime: AuthorizationGrantLifetime,
    ) -> Result<(), AuthorizationError> {
        let mut state = self
            .runtime
            .inner
            .lock()
            .map_err(|_| AuthorizationError::StorageUnavailable)?;
        let now = authorization_timestamp();
        state.grants.insert(
            runtime_key(subject, scope_revision),
            RuntimeGrant {
                capabilities: capabilities.clone(),
                lifetime,
                granted_at: now.clone(),
            },
        );
        push_runtime_decision(
            &mut state,
            subject,
            scope_revision,
            SecurityDecision::Grant,
            capabilities.len() as u32,
            Some(lifetime),
            now,
        );
        Ok(())
    }

    fn clear_runtime_grant(
        &self,
        subject: &AuthorizationSubject,
        scope_revision: &str,
        audit: bool,
    ) -> Result<(), AuthorizationError> {
        let mut state = self
            .runtime
            .inner
            .lock()
            .map_err(|_| AuthorizationError::StorageUnavailable)?;
        let removed = state.grants.remove(&runtime_key(subject, scope_revision));
        if audit && let Some(removed) = removed {
            push_runtime_decision(
                &mut state,
                subject,
                scope_revision,
                SecurityDecision::Revoke,
                0,
                Some(removed.lifetime),
                authorization_timestamp(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecuritySubjectKind {
    Plugin,
}

impl SecuritySubjectKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "plugin" => Some(Self::Plugin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityDecision {
    Grant,
    Revoke,
}

impl SecurityDecision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "grant" => Some(Self::Grant),
            "revoke" => Some(Self::Revoke),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecurityGrantView {
    pub subject_kind: SecuritySubjectKind,
    pub subject_id: String,
    pub scope_revision: String,
    pub capabilities: Vec<String>,
    pub granted_at: String,
    pub lifetime: AuthorizationGrantLifetime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecurityDecisionView {
    pub id: i64,
    pub subject_kind: SecuritySubjectKind,
    pub subject_id: String,
    pub scope_revision: String,
    pub decision: SecurityDecision,
    pub capability_count: u32,
    pub decided_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<AuthorizationGrantLifetime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecurityCentreStatus {
    pub legal: LegalStatus,
    pub active_grants: Vec<SecurityGrantView>,
    pub recent_decisions: Vec<SecurityDecisionView>,
    pub decision_retention_limit: usize,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SecurityCentreError {
    #[error("WyrmGrid could not read the local authorization record.")]
    StorageUnavailable,
    #[error("The local authorization record contains invalid data.")]
    InvalidRecord,
}

pub trait SecurityCentreRepository: Send + Sync + 'static {
    fn load_security_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, SecurityCentreError>;
    fn list_security_grants(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, SecurityCentreError>;
    fn list_security_decisions(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, SecurityCentreError>;
}

impl SecurityCentreRepository for Store {
    fn load_security_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, SecurityCentreError> {
        self.load_legal_preferences()
            .map_err(|_| SecurityCentreError::StorageUnavailable)
    }

    fn list_security_grants(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, SecurityCentreError> {
        self.list_active_authorization_grant_records(limit)
            .map_err(|_| SecurityCentreError::StorageUnavailable)
    }

    fn list_security_decisions(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, SecurityCentreError> {
        self.list_authorization_decision_records(limit)
            .map_err(|_| SecurityCentreError::StorageUnavailable)
    }
}

pub struct SecurityCentreService<R> {
    repository: R,
    runtime: AuthorizationRuntime,
}

impl<R: SecurityCentreRepository> SecurityCentreService<R> {
    pub fn new(repository: R) -> Self {
        Self::with_runtime(repository, AuthorizationRuntime::default())
    }

    pub fn with_runtime(repository: R, runtime: AuthorizationRuntime) -> Self {
        Self {
            repository,
            runtime,
        }
    }

    pub fn status(&self) -> Result<SecurityCentreStatus, SecurityCentreError> {
        let legal =
            legal_status_from_preferences(self.repository.load_security_legal_preferences()?);
        let grant_records = self
            .repository
            .list_security_grants(MAX_ACTIVE_GRANT_RECORDS + 1)?;
        if grant_records.len() > MAX_ACTIVE_GRANT_RECORDS {
            return Err(SecurityCentreError::InvalidRecord);
        }
        let mut active_grants = group_security_grants(grant_records)?;
        let (temporary_grants, temporary_decisions) = runtime_security_views(&self.runtime)?;
        active_grants.extend(temporary_grants);
        active_grants.sort_by(|left, right| {
            left.subject_id
                .cmp(&right.subject_id)
                .then_with(|| left.scope_revision.cmp(&right.scope_revision))
        });
        let mut recent_decisions = temporary_decisions;
        recent_decisions.extend(
            self.repository
                .list_security_decisions(MAX_VISIBLE_DECISIONS)?
                .into_iter()
                .map(validate_security_decision)
                .collect::<Result<Vec<_>, _>>()?,
        );
        recent_decisions.sort_by(|left, right| {
            security_timestamp_sort_key(&right.decided_at)
                .cmp(&security_timestamp_sort_key(&left.decided_at))
                .then_with(|| right.id.cmp(&left.id))
        });
        recent_decisions.truncate(MAX_VISIBLE_DECISIONS);

        Ok(SecurityCentreStatus {
            legal,
            active_grants,
            recent_decisions,
            decision_retention_limit: AUTHORIZATION_DECISION_RETENTION_LIMIT,
        })
    }
}

fn runtime_key(subject: &AuthorizationSubject, scope_revision: &str) -> RuntimeGrantKey {
    RuntimeGrantKey {
        subject_kind: subject.kind.as_str().to_owned(),
        subject_id: subject.id.clone(),
        scope_revision: scope_revision.to_owned(),
    }
}

fn authorization_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn push_runtime_decision(
    state: &mut AuthorizationRuntimeState,
    subject: &AuthorizationSubject,
    scope_revision: &str,
    decision: SecurityDecision,
    capability_count: u32,
    lifetime: Option<AuthorizationGrantLifetime>,
    decided_at: String,
) {
    state.next_decision_id = state.next_decision_id.saturating_sub(1);
    state.decisions.insert(
        0,
        RuntimeDecision {
            id: state.next_decision_id,
            subject_kind: subject.kind.as_str().to_owned(),
            subject_id: subject.id.clone(),
            scope_revision: scope_revision.to_owned(),
            decision,
            capability_count,
            lifetime,
            decided_at,
        },
    );
    state.decisions.truncate(MAX_VISIBLE_DECISIONS);
}

fn runtime_security_views(
    runtime: &AuthorizationRuntime,
) -> Result<(Vec<SecurityGrantView>, Vec<SecurityDecisionView>), SecurityCentreError> {
    let state = runtime
        .inner
        .lock()
        .map_err(|_| SecurityCentreError::StorageUnavailable)?;
    let grants = state
        .grants
        .iter()
        .map(|(key, grant)| {
            let subject_kind = SecuritySubjectKind::parse(&key.subject_kind)
                .ok_or(SecurityCentreError::InvalidRecord)?;
            Ok(SecurityGrantView {
                subject_kind,
                subject_id: key.subject_id.clone(),
                scope_revision: key.scope_revision.clone(),
                capabilities: grant.capabilities.iter().cloned().collect(),
                granted_at: grant.granted_at.clone(),
                lifetime: grant.lifetime,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let decisions = state
        .decisions
        .iter()
        .map(|decision| {
            let subject_kind = SecuritySubjectKind::parse(&decision.subject_kind)
                .ok_or(SecurityCentreError::InvalidRecord)?;
            Ok(SecurityDecisionView {
                id: decision.id,
                subject_kind,
                subject_id: decision.subject_id.clone(),
                scope_revision: decision.scope_revision.clone(),
                decision: decision.decision,
                capability_count: decision.capability_count,
                decided_at: decision.decided_at.clone(),
                lifetime: decision.lifetime,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((grants, decisions))
}

fn group_security_grants(
    records: Vec<AuthorizationGrantRecord>,
) -> Result<Vec<SecurityGrantView>, SecurityCentreError> {
    type GrantKey = (SecuritySubjectKind, String, String);
    let mut grouped: BTreeMap<GrantKey, (BTreeSet<String>, String)> = BTreeMap::new();
    for record in records {
        let subject_kind = SecuritySubjectKind::parse(&record.subject_kind)
            .ok_or(SecurityCentreError::InvalidRecord)?;
        let subject = AuthorizationSubject {
            kind: AuthorizationSubjectKind::Plugin,
            id: record.subject_id.clone(),
        };
        let capabilities = BTreeSet::from([record.capability.clone()]);
        validate_request(&subject, &record.scope_revision, &capabilities)
            .map_err(|_| SecurityCentreError::InvalidRecord)?;
        if !valid_security_timestamp(&record.granted_at) {
            return Err(SecurityCentreError::InvalidRecord);
        }

        let entry = grouped
            .entry((subject_kind, record.subject_id, record.scope_revision))
            .or_insert_with(|| (BTreeSet::new(), record.granted_at.clone()));
        entry.0.insert(record.capability);
        if record.granted_at < entry.1 {
            entry.1 = record.granted_at;
        }
    }

    grouped
        .into_iter()
        .map(
            |((subject_kind, subject_id, scope_revision), (capabilities, granted_at))| {
                if capabilities.len() > MAX_CAPABILITIES_PER_SUBJECT {
                    return Err(SecurityCentreError::InvalidRecord);
                }
                Ok(SecurityGrantView {
                    subject_kind,
                    subject_id,
                    scope_revision,
                    capabilities: capabilities.into_iter().collect(),
                    granted_at,
                    lifetime: AuthorizationGrantLifetime::Standing,
                })
            },
        )
        .collect()
}

fn validate_security_decision(
    record: AuthorizationDecisionRecord,
) -> Result<SecurityDecisionView, SecurityCentreError> {
    let subject_kind = SecuritySubjectKind::parse(&record.subject_kind)
        .ok_or(SecurityCentreError::InvalidRecord)?;
    let subject = AuthorizationSubject {
        kind: AuthorizationSubjectKind::Plugin,
        id: record.subject_id.clone(),
    };
    validate_subject(&subject).map_err(|_| SecurityCentreError::InvalidRecord)?;
    validate_scope_revision(&record.scope_revision)
        .map_err(|_| SecurityCentreError::InvalidRecord)?;
    let decision =
        SecurityDecision::parse(&record.decision).ok_or(SecurityCentreError::InvalidRecord)?;
    let count_is_valid = match decision {
        SecurityDecision::Grant => {
            (1..=MAX_CAPABILITIES_PER_SUBJECT as u32).contains(&record.capability_count)
        }
        SecurityDecision::Revoke => record.capability_count == 0,
    };
    if record.id <= 0 || !count_is_valid || !valid_security_timestamp(&record.decided_at) {
        return Err(SecurityCentreError::InvalidRecord);
    }

    Ok(SecurityDecisionView {
        id: record.id,
        subject_kind,
        subject_id: record.subject_id,
        scope_revision: record.scope_revision,
        decision,
        capability_count: record.capability_count,
        decided_at: record.decided_at,
        lifetime: (decision == SecurityDecision::Grant)
            .then_some(AuthorizationGrantLifetime::Standing),
    })
}

fn valid_security_timestamp(value: &str) -> bool {
    value.len() <= 64
        && (DateTime::parse_from_rfc3339(value).is_ok()
            || NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").is_ok())
}

fn security_timestamp_sort_key(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.timestamp())
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|value| value.and_utc().timestamp())
        })
        .unwrap_or(i64::MIN)
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

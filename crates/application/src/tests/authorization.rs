use super::*;

struct UnavailableAuthorizationRepository;

struct UnavailableSecurityRepository;

impl SecurityCentreRepository for UnavailableSecurityRepository {
    fn load_security_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, SecurityCentreError> {
        Err(SecurityCentreError::StorageUnavailable)
    }

    fn list_security_grants(
        &self,
        _limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, SecurityCentreError> {
        Err(SecurityCentreError::StorageUnavailable)
    }

    fn list_security_decisions(
        &self,
        _limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, SecurityCentreError> {
        Err(SecurityCentreError::StorageUnavailable)
    }
}

struct InvalidSecurityRepository;

impl SecurityCentreRepository for InvalidSecurityRepository {
    fn load_security_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, SecurityCentreError> {
        Ok(None)
    }

    fn list_security_grants(
        &self,
        _limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, SecurityCentreError> {
        Ok(vec![AuthorizationGrantRecord {
            subject_kind: "unknown".into(),
            subject_id: "org.example.weather".into(),
            scope_revision: "plugin:1.0.0:on_air_company_read".into(),
            capability: "on_air_company_read".into(),
            granted_at: "2026-07-15 00:00:00".into(),
        }])
    }

    fn list_security_decisions(
        &self,
        _limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, SecurityCentreError> {
        Ok(Vec::new())
    }
}

struct OversizedSecurityRepository;

impl SecurityCentreRepository for OversizedSecurityRepository {
    fn load_security_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, SecurityCentreError> {
        Ok(None)
    }

    fn list_security_grants(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, SecurityCentreError> {
        assert_eq!(limit, MAX_ACTIVE_GRANT_RECORDS + 1);
        Ok((0..limit)
            .map(|index| AuthorizationGrantRecord {
                subject_kind: "plugin".into(),
                subject_id: format!("org.example.weather-{index}"),
                scope_revision: "plugin:1.0.0:on_air_company_read".into(),
                capability: "on_air_company_read".into(),
                granted_at: "2026-07-15 00:00:00".into(),
            })
            .collect())
    }

    fn list_security_decisions(
        &self,
        _limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, SecurityCentreError> {
        panic!("decision history must not load after the active-grant bound fails")
    }
}

impl AuthorizationRepository for UnavailableAuthorizationRepository {
    fn list_grants(
        &self,
        _subject: &AuthorizationSubject,
        _scope_revision: &str,
    ) -> Result<Vec<String>, AuthorizationError> {
        Err(AuthorizationError::StorageUnavailable)
    }

    fn replace_grants(
        &self,
        _subject: &AuthorizationSubject,
        _scope_revision: &str,
        _capabilities: &[String],
    ) -> Result<(), AuthorizationError> {
        Err(AuthorizationError::StorageUnavailable)
    }
}

#[test]
fn grants_are_deny_by_default_and_bound_to_the_exact_revision() {
    let service = AuthorizationService::new(Store::open_in_memory().expect("store should open"));
    let subject = AuthorizationSubject::plugin("org.example.weather");
    let requested = BTreeSet::from(["on_air_company_read".to_owned()]);
    let first_revision = "plugin:1.0.0:on_air_company_read";

    assert!(
        service
            .grants(&subject, first_revision, &requested)
            .expect("empty grants should load")
            .is_empty()
    );
    assert_eq!(
        service.require_all(&subject, first_revision, &requested),
        Err(AuthorizationError::CapabilityRequired)
    );

    service
        .approve(&subject, first_revision, &requested)
        .expect("grant should approve");
    assert_eq!(
        service
            .require_all(&subject, first_revision, &requested)
            .expect("approved revision should authorize"),
        requested
    );
    assert_eq!(
        service.require_all(&subject, "plugin:1.1.0:on_air_company_read", &requested),
        Err(AuthorizationError::CapabilityRequired)
    );
}

#[test]
fn revocation_removes_every_capability_for_the_subject() {
    let service = AuthorizationService::new(Store::open_in_memory().expect("store should open"));
    let subject = AuthorizationSubject::plugin("org.example.weather");
    let revision = "plugin:1.0.0:external_network|on_air_company_read";
    let requested = BTreeSet::from([
        "external_network".to_owned(),
        "on_air_company_read".to_owned(),
    ]);
    service
        .approve(&subject, revision, &requested)
        .expect("grant should approve");

    service
        .revoke(&subject, revision)
        .expect("grant should revoke");
    assert_eq!(
        service.require_all(&subject, revision, &requested),
        Err(AuthorizationError::CapabilityRequired)
    );
}

#[test]
fn malformed_subjects_capabilities_and_revisions_are_rejected() {
    let service = AuthorizationService::new(Store::open_in_memory().expect("store should open"));
    let requested = BTreeSet::from(["on_air_company_read".to_owned()]);
    assert_eq!(
        service.approve(
            &AuthorizationSubject::plugin("../weather"),
            "plugin:1.0.0:on_air_company_read",
            &requested,
        ),
        Err(AuthorizationError::InvalidSubject)
    );
    assert_eq!(
        service.approve(
            &AuthorizationSubject::plugin("org.example.weather"),
            "bad revision with spaces",
            &requested,
        ),
        Err(AuthorizationError::InvalidScopeRevision)
    );
    assert_eq!(
        service.approve(
            &AuthorizationSubject::plugin("org.example.weather"),
            "plugin:1.0.0:unsafe",
            &BTreeSet::from(["Unsafe Capability".to_owned()]),
        ),
        Err(AuthorizationError::InvalidCapability)
    );
}

#[test]
fn unavailable_storage_fails_closed() {
    let service = AuthorizationService::new(UnavailableAuthorizationRepository);
    let subject = AuthorizationSubject::plugin("org.example.weather");
    let requested = BTreeSet::from(["on_air_company_read".to_owned()]);
    assert_eq!(
        service.require_all(&subject, "plugin:1.0.0:on_air_company_read", &requested,),
        Err(AuthorizationError::StorageUnavailable)
    );
    assert_eq!(
        service.approve(&subject, "plugin:1.0.0:on_air_company_read", &requested,),
        Err(AuthorizationError::StorageUnavailable)
    );
}

#[test]
fn security_centre_groups_active_grants_and_reports_recent_decisions() {
    let store = Store::open_in_memory().expect("store should open");
    LegalSettingsService::new(store.clone())
        .acknowledge(true)
        .expect("legal acknowledgement should save");
    let authorization = AuthorizationService::new(store.clone());
    let subject = AuthorizationSubject::plugin("org.example.weather");
    let revision = "plugin:1.0.0:external_network|on_air_company_read";
    authorization
        .approve(
            &subject,
            revision,
            &BTreeSet::from([
                "external_network".to_owned(),
                "on_air_company_read".to_owned(),
            ]),
        )
        .expect("grant should approve");

    let status = SecurityCentreService::new(store)
        .status()
        .expect("security status should load");
    assert!(status.legal.acknowledged);
    assert!(status.legal.telemetry_enabled);
    assert_eq!(status.active_grants.len(), 1);
    assert_eq!(status.active_grants[0].subject_id, "org.example.weather");
    assert_eq!(
        status.active_grants[0].capabilities,
        vec!["external_network", "on_air_company_read"]
    );
    assert_eq!(status.recent_decisions.len(), 1);
    assert_eq!(status.recent_decisions[0].decision, SecurityDecision::Grant);
    assert_eq!(status.recent_decisions[0].capability_count, 2);
    assert_eq!(
        status.decision_retention_limit,
        AUTHORIZATION_DECISION_RETENTION_LIMIT
    );
}

#[test]
fn security_centre_rejects_invalid_records_and_reports_unavailable_storage() {
    assert!(!valid_security_timestamp("Yesterday, probably"));
    assert!(valid_security_timestamp("2026-07-15T00:00:00Z"));
    assert!(valid_security_timestamp("2026-07-15 00:00:00"));
    assert_eq!(
        SecurityCentreService::new(InvalidSecurityRepository).status(),
        Err(SecurityCentreError::InvalidRecord)
    );
    assert_eq!(
        SecurityCentreService::new(UnavailableSecurityRepository).status(),
        Err(SecurityCentreError::StorageUnavailable)
    );
    assert_eq!(
        SecurityCentreService::new(OversizedSecurityRepository).status(),
        Err(SecurityCentreError::InvalidRecord)
    );
}

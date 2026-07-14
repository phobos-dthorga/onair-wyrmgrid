use super::*;

struct UnavailableAuthorizationRepository;

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

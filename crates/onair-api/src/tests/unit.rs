use super::*;

#[test]
fn debug_output_never_contains_the_api_key() {
    let client = OnAirClient::new(
        "https://server1.onair.company/api/v1",
        Uuid::nil(),
        SecretString::from("super-secret-key".to_owned()),
    )
    .expect("client should be valid");

    let debug = format!("{client:?}");
    assert!(!debug.contains("super-secret-key"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn company_requests_include_both_observed_authentication_headers() {
    let company_id = Uuid::parse_str("11111111-2222-4333-8444-555555555555")
        .expect("test company ID should be valid");
    let client = OnAirClient::new(
        "https://server1.onair.company/api/v1",
        company_id,
        SecretString::from("synthetic-test-key".to_owned()),
    )
    .expect("client should be valid");

    let request = client
        .request(&format!("company/{company_id}"))
        .expect("request should build");

    assert_eq!(
        request.headers().get(API_KEY_HEADER),
        Some(&header::HeaderValue::from_static("synthetic-test-key"))
    );
    assert_eq!(
        request.headers().get(COMPANY_ID_HEADER),
        Some(
            &header::HeaderValue::from_str(&company_id.to_string())
                .expect("company ID should be a valid header")
        )
    );
    assert_eq!(
        request.url().as_str(),
        format!("https://server1.onair.company/api/v1/company/{company_id}")
    );
}

#[test]
fn translates_the_swagger_company_envelope() {
    let response: ApiResult<RawCompany> = serde_json::from_str(include_str!(
        "../../tests/fixtures/swagger-company-response.json"
    ))
    .expect("synthetic Swagger fixture should deserialize");
    let company = response.content.expect("fixture should contain a company");

    assert_eq!(company.name, "Example Air");
    assert_eq!(company.airline_code, "WYR");
}

#[test]
fn translates_the_swagger_fleet_envelope_without_inventing_missing_facts() {
    let response: ApiResult<Vec<RawAircraft>> = serde_json::from_str(include_str!(
        "../../tests/fixtures/swagger-fleet-response.json"
    ))
    .expect("synthetic Swagger fixture should deserialize");
    let aircraft: Vec<_> = response
        .content
        .expect("fixture should contain a fleet")
        .into_iter()
        .filter_map(translate_aircraft)
        .collect();

    assert_eq!(aircraft.len(), 3);
    assert_eq!(aircraft[0].registration.as_deref(), Some("WYR-101"));
    assert_eq!(aircraft[0].model.as_deref(), Some("Example Turboprop"));
    assert_eq!(
        aircraft[0]
            .current_airport
            .as_ref()
            .and_then(|airport| airport.icao.as_deref()),
        Some("YTEST")
    );
    assert_eq!(
        aircraft[1].location,
        Some(Coordinates {
            latitude: -33.86,
            longitude: 151.2,
        })
    );
    assert_eq!(aircraft[2].registration, None);
    assert_eq!(aircraft[2].model, None);
    assert_eq!(aircraft[2].location, None);
}

#[test]
fn translates_the_swagger_fbo_envelope_without_inventing_missing_facts() {
    let response: ApiResult<Vec<RawFbo>> = serde_json::from_str(include_str!(
        "../../tests/fixtures/swagger-fbo-response.json"
    ))
    .expect("synthetic Swagger fixture should deserialize");
    let fbos: Vec<_> = response
        .content
        .expect("fixture should contain FBOs")
        .into_iter()
        .filter_map(translate_fbo)
        .collect();

    assert_eq!(fbos.len(), 3);
    assert_eq!(fbos[0].name.as_deref(), Some("WyrmGrid Test Base"));
    assert_eq!(
        fbos[0]
            .airport
            .as_ref()
            .and_then(|airport| airport.icao.as_deref()),
        Some("YTEST")
    );
    assert_eq!(
        fbos[0]
            .airport
            .as_ref()
            .and_then(|airport| airport.location),
        Some(Coordinates {
            latitude: -37.81,
            longitude: 144.96,
        })
    );
    assert_eq!(fbos[1].name, None);
    assert!(fbos[1].airport.is_some());
    assert_eq!(
        fbos[1]
            .airport
            .as_ref()
            .and_then(|airport| airport.location),
        None
    );
    assert_eq!(fbos[2].airport, None);
}

#[test]
fn translates_pending_jobs_into_the_stable_snapshot_contract() {
    let response: ApiResult<Vec<RawMission>> = serde_json::from_str(include_str!(
        "../../tests/fixtures/swagger-pending-jobs-response.json"
    ))
    .expect("synthetic Swagger fixture should deserialize");
    let snapshot = JobSnapshot {
        schema_version: JOB_SNAPSHOT_SCHEMA_VERSION,
        jobs: response
            .content
            .expect("fixture should contain missions")
            .into_iter()
            .filter_map(translate_job)
            .collect(),
    };

    snapshot.validate().expect("snapshot should validate");
    assert_eq!(snapshot.jobs.len(), 2);
    assert_eq!(snapshot.jobs[0].cargo_weight_lb(), Some(4_000.0));
    assert_eq!(snapshot.jobs[1].passenger_count(), Some(8));
    assert_eq!(
        snapshot.jobs[0]
            .route()
            .and_then(|(departure, _)| departure.icao.as_deref()),
        Some("YSSY")
    );
}

#[test]
fn translates_only_reviewed_bounded_staff_fields() {
    let response: ApiResult<Vec<RawStaffMember>> = serde_json::from_str(include_str!(
        "../../tests/fixtures/swagger-staff-response.json"
    ))
    .expect("synthetic Swagger fixture should deserialize");
    let snapshot = StaffSnapshot {
        schema_version: STAFF_SNAPSHOT_SCHEMA_VERSION,
        staff: response
            .content
            .expect("fixture should contain staff")
            .into_iter()
            .filter_map(translate_staff_member)
            .collect(),
    };

    snapshot.validate().expect("snapshot should validate");
    assert_eq!(snapshot.staff.len(), 2);
    assert_eq!(
        snapshot.staff[0].display_name.as_deref(),
        Some("Synthetic Pilot")
    );
    assert_eq!(
        snapshot.staff[0].avatar_reference.as_deref(),
        Some("synthetic-pilot-avatar.png")
    );
    assert_eq!(
        snapshot.staff[0]
            .current_airport
            .as_ref()
            .and_then(|airport| airport.icao.as_deref()),
        Some("YSSY")
    );
    assert_eq!(snapshot.staff[0].class_qualifications.len(), 1);
    assert_eq!(
        snapshot.staff[0].class_qualifications[0]
            .short_name
            .as_deref(),
        Some("MEP")
    );
    assert!(snapshot.staff[1].class_qualifications.is_empty());
    assert_eq!(snapshot.staff[1].current_airport, None);
}

#[test]
fn bounds_staff_qualifications_and_withholds_unknown_provider_codes() {
    let member = RawStaffMember {
        id: Some(Uuid::from_u128(1)),
        display_name: Some("Synthetic Boundary Aviator".into()),
        avatar_reference: Some("synthetic-boundary-avatar.png".into()),
        category_code: Some(99),
        status_code: Some(99),
        current_airport: None,
        home_airport: None,
        busy_until: None,
        is_online: None,
        class_certifications: (0..(MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER + 1))
            .map(|index| RawClassCertification {
                id: Some(Uuid::from_u128(100 + index as u128)),
                aircraft_class_id: Some(Uuid::from_u128(1_000 + index as u128)),
                aircraft_class: None,
                last_validated_at: None,
            })
            .collect(),
    };

    let translated = translate_staff_member(member).expect("bounded member should remain usable");
    assert_eq!(translated.category_code, None);
    assert_eq!(translated.status_code, None);
    assert_eq!(
        translated.class_qualifications.len(),
        MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER
    );
    translated
        .validate()
        .expect("translated member should validate");
}

#[test]
fn rejects_mismatched_staff_class_identity_without_guessing() {
    let member = RawStaffMember {
        id: Some(Uuid::from_u128(1)),
        display_name: Some("Synthetic Class Boundary Aviator".into()),
        avatar_reference: None,
        category_code: Some(1),
        status_code: Some(1),
        current_airport: None,
        home_airport: None,
        busy_until: None,
        is_online: None,
        class_certifications: vec![RawClassCertification {
            id: Some(Uuid::from_u128(2)),
            aircraft_class_id: Some(Uuid::from_u128(3)),
            aircraft_class: Some(RawAircraftClass {
                id: Some(Uuid::from_u128(4)),
                short_name: Some("MISMATCH".into()),
                name: Some("Synthetic mismatched class".into()),
            }),
            last_validated_at: None,
        }],
    };

    let translated = translate_staff_member(member).expect("member identity should remain usable");
    assert!(translated.class_qualifications.is_empty());
    translated
        .validate()
        .expect("member should remain valid without the ambiguous qualification");
}

#[test]
fn accepts_null_job_leg_collections_and_naive_onair_timestamps() {
    let response: ApiResult<Vec<RawMission>> = serde_json::from_str(
        r#"{
            "Content": [{
                "Id": "77777777-7777-4777-8777-777777777777",
                "CreationDate": "2026-07-15T10:30:45.123",
                "TakenDate": null,
                "ExpirationDate": "not-a-date",
                "Cargos": null,
                "Charters": null
            }],
            "Error": null
        }"#,
    )
    .expect("schema-compatible provider variations should deserialize");

    let mission = response
        .content
        .expect("fixture should contain missions")
        .into_iter()
        .next()
        .expect("fixture should contain one mission");
    assert_eq!(
        mission.creation_date,
        Some(
            DateTime::parse_from_rfc3339("2026-07-15T10:30:45.123Z")
                .expect("expected timestamp should parse")
                .with_timezone(&Utc)
        )
    );
    assert_eq!(mission.taken_date, None);
    assert_eq!(mission.expiration_date, None);
    assert!(mission.cargos.is_empty());
    assert!(mission.charters.is_empty());
}

#[test]
fn exposes_stable_diagnostic_codes_without_remote_details() {
    assert_eq!(
        ClientError::MalformedResponse.diagnostic_code(),
        "onair.malformed_response"
    );
    assert_eq!(
        ClientError::ResponseTooLarge.diagnostic_code(),
        "onair.response_too_large"
    );
}

#[test]
fn never_exposes_the_remote_error_body() {
    let response: ApiResult<RawCompany> =
        serde_json::from_str(r#"{"Content":null,"Error":"credential-specific remote detail"}"#)
            .expect("error envelope should deserialize");

    assert!(response.error.is_some());
    assert_eq!(
        ClientError::ApiRejected.to_string(),
        "OnAir rejected the request"
    );
    assert!(
        !ClientError::ApiRejected
            .to_string()
            .contains("credential-specific")
    );
}

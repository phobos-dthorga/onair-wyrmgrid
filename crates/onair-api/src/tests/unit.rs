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

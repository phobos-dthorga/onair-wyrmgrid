use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/sanitized-latest-ofp.json");

#[test]
fn validates_user_references_without_exposing_them() {
    assert!(UserReference::parse(UserReferenceKind::PilotId, "1234567").is_ok());
    assert!(UserReference::parse(UserReferenceKind::Username, "wyrm.pilot").is_ok());
    assert_eq!(
        UserReference::parse(UserReferenceKind::Username, "not valid!"),
        Err(ClientError::InvalidUserReference)
    );
}

#[test]
fn translates_the_sanitized_contract_fixture() {
    let retrieved_at = DateTime::from_timestamp(1_783_214_400, 0).unwrap();
    let snapshot = translate_latest_ofp(FIXTURE, retrieved_at).unwrap();

    assert_eq!(snapshot.airports.value.origin.icao, "YSSY");
    assert_eq!(snapshot.airports.value.destination.icao, "NZAA");
    assert_eq!(snapshot.airports.value.alternates[0].icao, "NZWN");
    assert_eq!(
        snapshot
            .aircraft
            .as_ref()
            .unwrap()
            .value
            .icao_type
            .as_deref(),
        Some("B738")
    );
    assert_eq!(snapshot.route.as_ref().unwrap().value.legs.len(), 3);
    assert_eq!(
        snapshot.weights.as_ref().unwrap().value.payload,
        Some(Mass {
            value: 14_820.0,
            unit: MassUnit::Kilograms,
        })
    );
    assert_eq!(
        snapshot.identity.provenance.kind,
        ProvenanceKind::ExternalCalculation
    );
    assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn rejects_malformed_and_oversized_payloads_without_echoing_them() {
    let canary = br#"{"private_route":"DO-NOT-REPORT"}"#;
    let error = translate_latest_ofp(canary, Utc::now()).unwrap_err();
    assert_eq!(error, ClientError::MalformedPlan);
    assert!(!error.to_string().contains("DO-NOT-REPORT"));

    let oversized = vec![b' '; MAX_RESPONSE_BYTES + 1];
    assert_eq!(
        translate_latest_ofp(&oversized, Utc::now()),
        Err(ClientError::ResponseTooLarge)
    );
}

#[tokio::test]
async fn requests_only_the_documented_json_latest_ofp_shape() {
    let (endpoint, request, server) =
        serve_once("200 OK", &["Content-Type: application/json"], FIXTURE);
    let client = SimBriefClient::with_endpoint(endpoint).unwrap();
    let reference = UserReference::parse(UserReferenceKind::PilotId, "1234567").unwrap();

    let snapshot = client.fetch_latest(&reference).await.unwrap();
    assert_eq!(snapshot.airports.value.origin.icao, "YSSY");
    let request = request.recv().unwrap();
    assert!(request.starts_with("GET /latest?userid=1234567&json=1 HTTP/1.1"));
    assert!(!request.to_ascii_lowercase().contains("authorization:"));
    server.join().unwrap();
}

#[tokio::test]
async fn classifies_provider_errors_without_reading_or_echoing_the_body() {
    let (endpoint, _request, server) = serve_once(
        "400 Bad Request",
        &["Content-Type: application/json"],
        br#"{"error":"PRIVATE-CANARY"}"#,
    );
    let client = SimBriefClient::with_endpoint(endpoint).unwrap();
    let reference = UserReference::parse(UserReferenceKind::Username, "wyrm.pilot").unwrap();

    let error = client.fetch_latest(&reference).await.unwrap_err();
    assert_eq!(error, ClientError::NoPlan);
    assert!(!error.to_string().contains("PRIVATE-CANARY"));
    server.join().unwrap();
}

#[tokio::test]
async fn rejects_redirects_and_declared_oversized_responses() {
    let (endpoint, _request, server) = serve_once(
        "302 Found",
        &["Location: https://example.invalid/private"],
        b"",
    );
    let client = SimBriefClient::with_endpoint(endpoint).unwrap();
    let reference = UserReference::parse(UserReferenceKind::PilotId, "1234567").unwrap();
    assert_eq!(
        client.fetch_latest(&reference).await,
        Err(ClientError::UnexpectedResponse)
    );
    server.join().unwrap();

    let declared_length = format!("Content-Length: {}", MAX_RESPONSE_BYTES + 1);
    let (endpoint, _request, server) = serve_once(
        "200 OK",
        &["Content-Type: application/json", &declared_length],
        b"",
    );
    let client = SimBriefClient::with_endpoint(endpoint).unwrap();
    assert_eq!(
        client.fetch_latest(&reference).await,
        Err(ClientError::ResponseTooLarge)
    );
    server.join().unwrap();
}

fn serve_once(
    status: &str,
    headers: &[&str],
    body: &[u8],
) -> (Url, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let endpoint = Url::parse(&format!("http://{address}/latest")).unwrap();
    let (request_sender, request_receiver) = mpsc::channel();
    let status = status.to_owned();
    let headers = headers
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let body = body.to_vec();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let length = stream.read(&mut request).unwrap();
        request_sender
            .send(String::from_utf8_lossy(&request[..length]).into_owned())
            .unwrap();
        let has_length = headers
            .iter()
            .any(|header| header.to_ascii_lowercase().starts_with("content-length:"));
        let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
        for header in headers {
            response.push_str(&header);
            response.push_str("\r\n");
        }
        if !has_length {
            response.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        response.push_str("\r\n");
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
    });
    (endpoint, request_receiver, server)
}

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

const METAR_FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/sanitized-metars.json");
const TAF_FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/sanitized-tafs.json");

#[test]
fn validates_and_normalizes_a_bounded_station_set() {
    assert_eq!(
        normalize_stations(&[" yssy ".into(), "NZAA".into(), "YSSY".into()]).unwrap(),
        vec!["NZAA", "YSSY"]
    );
    assert_eq!(normalize_stations(&[]), Err(ClientError::InvalidStations));
    assert_eq!(
        normalize_stations(&["NOT-ICAO".into()]),
        Err(ClientError::InvalidStations)
    );
}

#[test]
fn translates_sanitized_metar_and_taf_fixtures() {
    let retrieved_at = DateTime::from_timestamp(1_783_993_200, 0).unwrap();
    let snapshot = translate_airport_weather(
        &["YSSY".into(), "NZAA".into(), "NZWN".into()],
        METAR_FIXTURE,
        TAF_FIXTURE,
        retrieved_at,
    )
    .unwrap();

    assert_eq!(snapshot.airports.len(), 3);
    let wellington = snapshot
        .airports
        .iter()
        .find(|airport| airport.station_icao == "NZWN")
        .unwrap();
    let metar = wellington.metar.as_ref().unwrap();
    assert_eq!(metar.value.flight_category, Some(FlightCategory::Mvfr));
    assert_eq!(metar.value.wind_gust_kt, Some(28));
    assert_eq!(metar.provenance.kind, ProvenanceKind::ExternalFact);
    assert!(wellington.taf.is_some());
    assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn rejects_malformed_and_oversized_payloads_without_echoing_them() {
    let canary = br#"[{"private":"DO-NOT-REPORT"}]"#;
    let error =
        translate_airport_weather(&["YSSY".into()], canary, TAF_FIXTURE, Utc::now()).unwrap_err();
    assert_eq!(error, ClientError::MalformedWeather);
    assert!(!error.to_string().contains("DO-NOT-REPORT"));

    let oversized = vec![b' '; MAX_RESPONSE_BYTES + 1];
    assert_eq!(
        translate_airport_weather(&["YSSY".into()], &oversized, b"[]", Utc::now()),
        Err(ClientError::ResponseTooLarge)
    );
}

#[tokio::test]
async fn requests_only_bounded_documented_json_products_without_authentication() {
    let responses = vec![
        response("200 OK", &["Content-Type: application/json"], METAR_FIXTURE),
        response("200 OK", &["Content-Type: application/json"], TAF_FIXTURE),
    ];
    let (base_url, requests, server) = serve_sequence(responses);
    let client = AviationWeatherClient::with_base_url(base_url).unwrap();
    let snapshot = client
        .fetch_airports(&["YSSY".into(), "NZAA".into(), "NZWN".into()])
        .await
        .unwrap();
    assert_eq!(snapshot.airports.len(), 3);

    let requests = requests.recv().unwrap();
    assert!(
        requests[0].starts_with("GET /api/data/metar?ids=NZAA%2CNZWN%2CYSSY&format=json HTTP/1.1")
    );
    assert!(
        requests[1].starts_with("GET /api/data/taf?ids=NZAA%2CNZWN%2CYSSY&format=json HTTP/1.1")
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.to_ascii_lowercase().contains("authorization:"))
    );
    server.join().unwrap();
}

#[tokio::test]
async fn accepts_no_content_and_classifies_provider_errors_without_bodies() {
    let responses = vec![
        response("204 No Content", &[], b""),
        response("200 OK", &["Content-Type: application/json"], TAF_FIXTURE),
    ];
    let (base_url, _requests, server) = serve_sequence(responses);
    let client = AviationWeatherClient::with_base_url(base_url).unwrap();
    let snapshot = client.fetch_airports(&["YSSY".into()]).await.unwrap();
    assert!(snapshot.airports[0].metar.is_none());
    assert!(snapshot.airports[0].taf.is_some());
    server.join().unwrap();

    let responses = vec![response(
        "429 Too Many Requests",
        &["Content-Type: application/json"],
        br#"{"error":"PRIVATE-CANARY"}"#,
    )];
    let (base_url, _requests, server) = serve_sequence(responses);
    let client = AviationWeatherClient::with_base_url(base_url).unwrap();
    let error = client.fetch_airports(&["YSSY".into()]).await.unwrap_err();
    assert_eq!(error, ClientError::RateLimited);
    assert!(!error.to_string().contains("PRIVATE-CANARY"));
    server.join().unwrap();
}

#[tokio::test]
async fn refuses_redirects_and_declared_oversized_responses() {
    let responses = vec![response(
        "302 Found",
        &["Location: https://example.invalid/private"],
        b"",
    )];
    let (base_url, requests, server) = serve_sequence(responses);
    let client = AviationWeatherClient::with_base_url(base_url).unwrap();
    assert_eq!(
        client.fetch_airports(&["YSSY".into()]).await,
        Err(ClientError::UnexpectedResponse)
    );
    assert_eq!(requests.recv().unwrap().len(), 1);
    server.join().unwrap();

    let responses = vec![response(
        "200 OK",
        &["Content-Type: application/json", "Content-Length: 524289"],
        b"",
    )];
    let (base_url, _requests, server) = serve_sequence(responses);
    let client = AviationWeatherClient::with_base_url(base_url).unwrap();
    assert_eq!(
        client.fetch_airports(&["YSSY".into()]).await,
        Err(ClientError::ResponseTooLarge)
    );
    server.join().unwrap();
}

fn response(status: &str, headers: &[&str], body: &[u8]) -> Vec<u8> {
    let has_length = headers
        .iter()
        .any(|header| header.to_ascii_lowercase().starts_with("content-length:"));
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    if !has_length {
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn serve_sequence(
    responses: Vec<Vec<u8>>,
) -> (Url, mpsc::Receiver<Vec<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let base_url = Url::parse(&format!("http://{address}/api/data/")).unwrap();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 8_192];
            let length = stream.read(&mut request).unwrap();
            requests.push(String::from_utf8_lossy(&request[..length]).into_owned());
            stream.write_all(&response).unwrap();
        }
        sender.send(requests).unwrap();
    });
    (base_url, receiver, server)
}

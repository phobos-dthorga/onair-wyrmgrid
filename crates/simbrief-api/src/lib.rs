//! Read-only, bounded SimBrief latest-OFP adapter.
//!
//! Raw provider JSON is parsed and translated here. It must not cross this
//! crate's public boundary or appear in errors, logs, telemetry, or plugins.

use chrono::{DateTime, Utc};
use reqwest::{StatusCode, header, redirect::Policy};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use wyrmgrid_domain::{
    Coordinates, FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION, FlightPlanAirport, FlightPlanAirports,
    FlightPlanIdentity, FlightPlanSnapshot, FlightPlanSnapshotId, Mass, MassUnit,
    OperationalObservation, OperationalProvenance, PlannedAircraft, PlannedFuel, PlannedRoute,
    PlannedRouteLeg, PlannedSchedule, PlannedWeights, ProvenanceKind, SnapshotFreshness,
};

pub const DEFAULT_ENDPOINT: &str = "https://www.simbrief.com/api/xml.fetcher.php";
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const TRANSFORMATION_VERSION: u32 = 1;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserReferenceKind {
    PilotId,
    Username,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserReference {
    kind: UserReferenceKind,
    value: String,
}

impl UserReference {
    pub fn parse(kind: UserReferenceKind, value: &str) -> Result<Self, ClientError> {
        let value = value.trim();
        let valid = match kind {
            UserReferenceKind::PilotId => {
                (1..=12).contains(&value.len()) && value.chars().all(|value| value.is_ascii_digit())
            }
            UserReferenceKind::Username => {
                (2..=64).contains(&value.len())
                    && value.chars().all(|value| {
                        value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | '.')
                    })
            }
        };
        if !valid {
            return Err(ClientError::InvalidUserReference);
        }
        Ok(Self {
            kind,
            value: value.to_owned(),
        })
    }

    fn query_name(&self) -> &'static str {
        match self.kind {
            UserReferenceKind::PilotId => "userid",
            UserReferenceKind::Username => "username",
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    #[error("The SimBrief importer could not be initialized.")]
    ConfigurationUnavailable,
    #[error("Enter a valid SimBrief Pilot ID or username.")]
    InvalidUserReference,
    #[error("SimBrief did not return a latest flight plan for that account reference.")]
    NoPlan,
    #[error("SimBrief is rate-limiting requests. Wait before trying again.")]
    RateLimited,
    #[error("The SimBrief request timed out.")]
    TimedOut,
    #[error("SimBrief is unreachable from this device right now.")]
    Offline,
    #[error("SimBrief is temporarily unavailable.")]
    ProviderUnavailable,
    #[error("SimBrief returned an unexpected response.")]
    UnexpectedResponse,
    #[error("The SimBrief response exceeded WyrmGrid's 2 MiB safety limit.")]
    ResponseTooLarge,
    #[error("SimBrief returned a response that was not JSON.")]
    InvalidContentType,
    #[error("The latest SimBrief plan did not match WyrmGrid's validated import contract.")]
    MalformedPlan,
}

#[derive(Clone)]
pub struct SimBriefClient {
    http: reqwest::Client,
    endpoint: Url,
}

impl SimBriefClient {
    pub fn new() -> Result<Self, ClientError> {
        let endpoint =
            Url::parse(DEFAULT_ENDPOINT).map_err(|_| ClientError::ConfigurationUnavailable)?;
        Self::with_endpoint(endpoint)
    }

    fn with_endpoint(endpoint: Url) -> Result<Self, ClientError> {
        if endpoint.scheme() != "https" && !cfg!(test) {
            return Err(ClientError::ConfigurationUnavailable);
        }
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("OnAir-WyrmGrid/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| ClientError::ConfigurationUnavailable)?;
        Ok(Self { http, endpoint })
    }

    pub async fn fetch_latest(
        &self,
        reference: &UserReference,
    ) -> Result<FlightPlanSnapshot, ClientError> {
        let mut request_url = self.endpoint.clone();
        request_url
            .query_pairs_mut()
            .append_pair(reference.query_name(), &reference.value)
            .append_pair("json", "1");

        let response = self
            .http
            .get(request_url)
            .send()
            .await
            .map_err(classify_transport_error)?;
        let status = response.status();
        if status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND {
            return Err(ClientError::NoPlan);
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(ClientError::RateLimited);
        }
        if status.is_redirection() {
            return Err(ClientError::UnexpectedResponse);
        }
        if status.is_server_error() {
            return Err(ClientError::ProviderUnavailable);
        }
        if status != StatusCode::OK {
            return Err(ClientError::UnexpectedResponse);
        }

        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::ResponseTooLarge);
        }
        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            let content_type = content_type
                .to_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !content_type.starts_with("application/json")
                && !content_type.starts_with("text/json")
                && !content_type.starts_with("text/plain")
            {
                return Err(ClientError::InvalidContentType);
            }
        }

        let mut response = response;
        let mut bytes = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(16 * 1024)
                .min(MAX_RESPONSE_BYTES as u64) as usize,
        );
        while let Some(chunk) = response.chunk().await.map_err(classify_transport_error)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(ClientError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }

        translate_latest_ofp(&bytes, Utc::now())
    }
}

fn classify_transport_error(error: reqwest::Error) -> ClientError {
    if error.is_timeout() {
        ClientError::TimedOut
    } else if error.is_connect() {
        ClientError::Offline
    } else {
        ClientError::ProviderUnavailable
    }
}

pub fn translate_latest_ofp(
    bytes: &[u8],
    retrieved_at: DateTime<Utc>,
) -> Result<FlightPlanSnapshot, ClientError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(ClientError::ResponseTooLarge);
    }
    let root: Value = serde_json::from_slice(bytes).map_err(|_| ClientError::MalformedPlan)?;
    if !root.is_object() {
        return Err(ClientError::MalformedPlan);
    }
    if text_at(&root, &["fetch", "status"])
        .is_some_and(|status| !status.eq_ignore_ascii_case("success"))
    {
        return Err(ClientError::NoPlan);
    }

    let generated_at = timestamp_at(&root, &["params", "time_generated"])
        .or_else(|| timestamp_at(&root, &["general", "generated"]));
    let airac = text_at(&root, &["general", "airac"]);
    let provenance = OperationalProvenance {
        kind: ProvenanceKind::ExternalCalculation,
        provider: "simbrief".into(),
        provider_revision: airac.clone(),
        generated_at,
        retrieved_at,
        transformation_version: TRANSFORMATION_VERSION,
        freshness: SnapshotFreshness::Current,
    };

    let airports = FlightPlanAirports {
        origin: airport_at(&root, "origin").ok_or(ClientError::MalformedPlan)?,
        destination: airport_at(&root, "destination").ok_or(ClientError::MalformedPlan)?,
        alternates: alternates_at(&root),
    };
    let aircraft = aircraft_at(&root).map(|value| observation(value, &provenance));
    let schedule = schedule_at(&root).map(|value| observation(value, &provenance));
    let unit = mass_unit_at(&root);
    let weights = unit
        .and_then(|unit| weights_at(&root, unit))
        .map(|value| observation(value, &provenance));
    let fuel = unit
        .and_then(|unit| fuel_at(&root, unit))
        .map(|value| observation(value, &provenance));
    let route = route_at(&root).map(|value| observation(value, &provenance));

    let snapshot = FlightPlanSnapshot {
        schema_version: FLIGHT_PLAN_SNAPSHOT_SCHEMA_VERSION,
        id: FlightPlanSnapshotId(Uuid::new_v4()),
        identity: observation(
            FlightPlanIdentity {
                airac,
                provider_plan_reference: text_at(&root, &["fetch", "static_id"]),
            },
            &provenance,
        ),
        airports: observation(airports, &provenance),
        aircraft,
        schedule,
        weights,
        fuel,
        route,
    };
    snapshot
        .validate()
        .map_err(|_| ClientError::MalformedPlan)?;
    Ok(snapshot)
}

fn observation<T>(value: T, provenance: &OperationalProvenance) -> OperationalObservation<T> {
    OperationalObservation {
        value,
        provenance: provenance.clone(),
    }
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(root, |value, key| value.get(*key))
}

fn text_at(root: &Value, path: &[&str]) -> Option<String> {
    let value = value_at(root, path)?;
    let value = match value {
        Value::String(value) => value.trim().to_owned(),
        Value::Number(value) => value.to_string(),
        _ => return None,
    };
    (!value.is_empty()).then_some(value)
}

fn number_at(root: &Value, path: &[&str]) -> Option<f64> {
    let value = value_at(root, path)?;
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
    .filter(|value| value.is_finite())
}

fn unsigned_at(root: &Value, path: &[&str]) -> Option<u32> {
    let value = number_at(root, path)?;
    (value.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&value)).then_some(value as u32)
}

fn timestamp_at(root: &Value, path: &[&str]) -> Option<DateTime<Utc>> {
    let value = value_at(root, path)?;
    let seconds = match value {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }?;
    DateTime::from_timestamp(seconds, 0)
}

fn airport_at(root: &Value, section: &str) -> Option<FlightPlanAirport> {
    let airport = root.get(section)?;
    airport_from_value(airport)
}

fn airport_from_value(airport: &Value) -> Option<FlightPlanAirport> {
    let icao = text_at(airport, &["icao_code"])?.to_ascii_uppercase();
    let latitude = number_at(airport, &["pos_lat"]);
    let longitude = number_at(airport, &["pos_long"]);
    let location = latitude.zip(longitude).and_then(|(latitude, longitude)| {
        let coordinates = Coordinates {
            latitude,
            longitude,
        };
        coordinates.is_valid().then_some(coordinates)
    });
    Some(FlightPlanAirport {
        icao,
        name: text_at(airport, &["name"]),
        location,
        planned_runway: text_at(airport, &["plan_rwy"]).map(|value| value.to_ascii_uppercase()),
    })
}

fn alternates_at(root: &Value) -> Vec<FlightPlanAirport> {
    let Some(value) = root.get("alternates").or_else(|| root.get("alternate")) else {
        return Vec::new();
    };
    match value {
        Value::Array(values) => values.iter().filter_map(airport_from_value).collect(),
        Value::Object(_) => airport_from_value(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn aircraft_at(root: &Value) -> Option<PlannedAircraft> {
    let aircraft = root.get("aircraft")?;
    let value = PlannedAircraft {
        icao_type: text_at(aircraft, &["icaocode"]).map(|value| value.to_ascii_uppercase()),
        registration: text_at(aircraft, &["reg"]),
        model: text_at(aircraft, &["name"]),
    };
    (value.icao_type.is_some() || value.registration.is_some() || value.model.is_some())
        .then_some(value)
}

fn schedule_at(root: &Value) -> Option<PlannedSchedule> {
    let value = PlannedSchedule {
        scheduled_out: timestamp_at(root, &["times", "sched_out"]),
        scheduled_off: timestamp_at(root, &["times", "sched_off"]),
        scheduled_on: timestamp_at(root, &["times", "sched_on"]),
        scheduled_in: timestamp_at(root, &["times", "sched_in"]),
        estimated_enroute_seconds: unsigned_at(root, &["times", "est_time_enroute"]),
    };
    (value.scheduled_out.is_some()
        || value.scheduled_off.is_some()
        || value.scheduled_on.is_some()
        || value.scheduled_in.is_some()
        || value.estimated_enroute_seconds.is_some())
    .then_some(value)
}

fn mass_unit_at(root: &Value) -> Option<MassUnit> {
    let unit = text_at(root, &["params", "units"])
        .or_else(|| text_at(root, &["weights", "units"]))?
        .to_ascii_lowercase();
    match unit.as_str() {
        "kg" | "kgs" | "kilogram" | "kilograms" => Some(MassUnit::Kilograms),
        "lb" | "lbs" | "pound" | "pounds" => Some(MassUnit::Pounds),
        _ => None,
    }
}

fn mass_at(root: &Value, path: &[&str], unit: MassUnit) -> Option<Mass> {
    number_at(root, path).map(|value| Mass { value, unit })
}

fn weights_at(root: &Value, unit: MassUnit) -> Option<PlannedWeights> {
    let value = PlannedWeights {
        payload: mass_at(root, &["weights", "payload"], unit),
        zero_fuel: mass_at(root, &["weights", "est_zfw"], unit),
        takeoff: mass_at(root, &["weights", "est_tow"], unit),
        landing: mass_at(root, &["weights", "est_ldw"], unit),
    };
    (value.payload.is_some()
        || value.zero_fuel.is_some()
        || value.takeoff.is_some()
        || value.landing.is_some())
    .then_some(value)
}

fn fuel_at(root: &Value, unit: MassUnit) -> Option<PlannedFuel> {
    let value = PlannedFuel {
        taxi: mass_at(root, &["fuel", "taxi"], unit),
        enroute: mass_at(root, &["fuel", "enroute_burn"], unit),
        reserve: mass_at(root, &["fuel", "reserve"], unit),
        alternate: mass_at(root, &["fuel", "alternate_burn"], unit),
        contingency: mass_at(root, &["fuel", "contingency"], unit),
        extra: mass_at(root, &["fuel", "extra"], unit),
        ramp: mass_at(root, &["fuel", "plan_ramp"], unit),
        takeoff: mass_at(root, &["fuel", "plan_takeoff"], unit),
        landing: mass_at(root, &["fuel", "plan_landing"], unit),
    };
    [
        value.taxi,
        value.enroute,
        value.reserve,
        value.alternate,
        value.contingency,
        value.extra,
        value.ramp,
        value.takeoff,
        value.landing,
    ]
    .iter()
    .any(Option::is_some)
    .then_some(value)
}

fn route_at(root: &Value) -> Option<PlannedRoute> {
    let legs = value_at(root, &["navlog", "fix"])
        .map(route_legs)
        .unwrap_or_default();
    let value = PlannedRoute {
        source_text: text_at(root, &["general", "route"]),
        initial_altitude_ft: unsigned_at(root, &["general", "initial_altitude"]),
        distance_nm: number_at(root, &["general", "route_distance"])
            .or_else(|| number_at(root, &["general", "distance"])),
        legs,
    };
    (value.source_text.is_some()
        || value.initial_altitude_ft.is_some()
        || value.distance_nm.is_some()
        || !value.legs.is_empty())
    .then_some(value)
}

fn route_legs(value: &Value) -> Vec<PlannedRouteLeg> {
    let values = match value {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    values
        .into_iter()
        .filter_map(|value| {
            let ident = text_at(value, &["ident"])?.to_ascii_uppercase();
            let latitude = number_at(value, &["pos_lat"]);
            let longitude = number_at(value, &["pos_long"]);
            let location = latitude.zip(longitude).and_then(|(latitude, longitude)| {
                let coordinates = Coordinates {
                    latitude,
                    longitude,
                };
                coordinates.is_valid().then_some(coordinates)
            });
            Some((
                ident,
                text_at(value, &["via_airway"]).map(|value| value.to_ascii_uppercase()),
                location,
            ))
        })
        .enumerate()
        .map(|(sequence, (ident, airway, location))| PlannedRouteLeg {
            sequence: sequence as u32,
            ident,
            airway,
            location,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/sanitized-latest-ofp.json");

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
}

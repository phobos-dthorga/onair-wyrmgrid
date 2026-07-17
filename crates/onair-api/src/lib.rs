//! Read-only OnAir API boundary.
//!
//! Raw responses stay inside this crate. Other parts of WyrmGrid consume stable
//! domain models rather than binding themselves to OnAir's JSON field names.

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use std::collections::HashSet;
use std::time::Duration;
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftClassId, AircraftClassQualification, AircraftId, AircraftSummary, AirportId,
    AirportSummary, CompanyId, CompanySummary, Coordinates, FboId, FboSummary,
    JOB_SNAPSHOT_SCHEMA_VERSION, JobId, JobLeg, JobLegId, JobLegKind, JobSnapshot, JobSummary,
    MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER, MAX_JOBS_PER_SNAPSHOT, MAX_STAFF_PER_SNAPSHOT,
    Observed, Provenance, ProvenanceKind, STAFF_SNAPSHOT_SCHEMA_VERSION, StaffMemberId,
    StaffMemberSummary, StaffQualificationId, StaffSnapshot,
};

const API_KEY_HEADER: &str = "oa-apikey";
const COMPANY_ID_HEADER: &str = "CompanyUniqueId";
const FLEET_PROVENANCE_SOURCE: &str = "onair:company/fleet";
const FBOS_PROVENANCE_SOURCE: &str = "onair:company/fbos";
const JOBS_PROVENANCE_SOURCE: &str = "onair:company/jobs/pending";
const STAFF_PROVENANCE_SOURCE: &str = "onair:company/employees";
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
pub const DEFAULT_BASE_URL: &str = "https://server1.onair.company/api/v1";

pub struct OnAirClient {
    http: Client,
    base_url: Url,
    company_id: Uuid,
    api_key: SecretString,
}

impl std::fmt::Debug for OnAirClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OnAirClient")
            .field("base_url", &self.base_url)
            .field("company_id", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

impl ClientError {
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::InvalidBaseUrl(_) => "onair.invalid_base_url",
            Self::InvalidRequestHeader(_) => "onair.invalid_request_header",
            Self::Transport(error) if error.is_timeout() => "onair.request_timeout",
            Self::Transport(_) => "onair.transport_failure",
            Self::Http(_) => "onair.http_failure",
            Self::AuthenticationRejected => "onair.authentication_rejected",
            Self::CompanyNotFound => "onair.company_not_found",
            Self::RateLimited => "onair.rate_limited",
            Self::ApiRejected => "onair.api_rejected",
            Self::MissingContent => "onair.missing_content",
            Self::ResponseTooLarge => "onair.response_too_large",
            Self::InvalidContentType => "onair.invalid_content_type",
            Self::MalformedResponse => "onair.malformed_response",
        }
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid OnAir API base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("invalid OnAir request header")]
    InvalidRequestHeader(#[from] header::InvalidHeaderValue),
    #[error("OnAir request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("OnAir returned HTTP {0}")]
    Http(StatusCode),
    #[error("OnAir rejected the API key or company ID")]
    AuthenticationRejected,
    #[error("OnAir company was not found")]
    CompanyNotFound,
    #[error("OnAir rate limit reached")]
    RateLimited,
    #[error("OnAir rejected the request")]
    ApiRejected,
    #[error("OnAir returned an incomplete company response")]
    MissingContent,
    #[error("OnAir returned a response larger than WyrmGrid accepts")]
    ResponseTooLarge,
    #[error("OnAir returned an unexpected content type")]
    InvalidContentType,
    #[error("OnAir returned malformed JSON")]
    MalformedResponse,
}

#[derive(Debug, Deserialize)]
struct ApiResult<T> {
    #[serde(rename = "Content")]
    content: Option<T>,
    #[serde(rename = "Error", default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCompany {
    #[serde(rename = "Id")]
    id: Uuid,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AirlineCode", default)]
    airline_code: String,
}

#[derive(Debug, Deserialize)]
struct RawAircraft {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "Identifier", default)]
    identifier: Option<String>,
    #[serde(rename = "AircraftType", default)]
    aircraft_type: Option<RawAircraftType>,
    #[serde(rename = "Longitude", default)]
    longitude: Option<f64>,
    #[serde(rename = "Latitude", default)]
    latitude: Option<f64>,
    #[serde(rename = "CurrentAirport", default)]
    current_airport: Option<RawAirport>,
}

#[derive(Debug, Deserialize)]
struct RawAircraftType {
    #[serde(rename = "DisplayName", default)]
    display_name: Option<String>,
    #[serde(rename = "TypeName", default)]
    type_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAirport {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "ICAO", default)]
    icao: Option<String>,
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "Longitude", default)]
    longitude: Option<f64>,
    #[serde(rename = "Latitude", default)]
    latitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawFbo {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "Airport", default)]
    airport: Option<RawAirport>,
    #[serde(rename = "AirportId", default)]
    airport_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RawMission {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "MissionType", default)]
    mission_type: Option<RawMissionType>,
    #[serde(rename = "Description", default)]
    description: Option<String>,
    #[serde(rename = "Pay", default)]
    pay: Option<f64>,
    #[serde(
        rename = "CreationDate",
        default,
        deserialize_with = "deserialize_onair_datetime"
    )]
    creation_date: Option<DateTime<Utc>>,
    #[serde(
        rename = "TakenDate",
        default,
        deserialize_with = "deserialize_onair_datetime"
    )]
    taken_date: Option<DateTime<Utc>>,
    #[serde(
        rename = "ExpirationDate",
        default,
        deserialize_with = "deserialize_onair_datetime"
    )]
    expiration_date: Option<DateTime<Utc>>,
    #[serde(
        rename = "Cargos",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    cargos: Vec<RawCargo>,
    #[serde(
        rename = "Charters",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    charters: Vec<RawCharter>,
}

#[derive(Debug, Deserialize)]
struct RawMissionType {
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "ShortName", default)]
    short_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCargo {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "LegIndex", default)]
    leg_index: Option<u32>,
    #[serde(rename = "Weight", default)]
    weight: Option<f64>,
    #[serde(rename = "Distance", default)]
    distance: Option<f64>,
    #[serde(rename = "Description", default)]
    description: Option<String>,
    #[serde(rename = "DepartureAirport", default)]
    departure_airport: Option<RawAirport>,
    #[serde(rename = "DestinationAirport", default)]
    destination_airport: Option<RawAirport>,
    #[serde(rename = "CurrentAirport", default)]
    current_airport: Option<RawAirport>,
}

#[derive(Debug, Deserialize)]
struct RawCharter {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "LegIndex", default)]
    leg_index: Option<u32>,
    #[serde(rename = "PassengersNumber", default)]
    passengers: Option<i64>,
    #[serde(rename = "Distance", default)]
    distance: Option<f64>,
    #[serde(rename = "Description", default)]
    description: Option<String>,
    #[serde(rename = "DepartureAirport", default)]
    departure_airport: Option<RawAirport>,
    #[serde(rename = "DestinationAirport", default)]
    destination_airport: Option<RawAirport>,
    #[serde(rename = "CurrentAirport", default)]
    current_airport: Option<RawAirport>,
}

#[derive(Debug, Deserialize)]
struct RawStaffMember {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "Pseudo", default)]
    display_name: Option<String>,
    #[serde(rename = "AvatarImageName", default)]
    avatar_reference: Option<String>,
    #[serde(rename = "Category", default)]
    category_code: Option<i32>,
    #[serde(rename = "Status", default)]
    status_code: Option<i32>,
    #[serde(rename = "CurrentAirport", default)]
    current_airport: Option<RawAirport>,
    #[serde(rename = "HomeAirport", default)]
    home_airport: Option<RawAirport>,
    #[serde(
        rename = "BusyUntil",
        default,
        deserialize_with = "deserialize_onair_datetime"
    )]
    busy_until: Option<DateTime<Utc>>,
    #[serde(rename = "IsOnline", default)]
    is_online: Option<bool>,
    #[serde(
        rename = "ClassCertifications",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    class_certifications: Vec<RawClassCertification>,
}

#[derive(Debug, Deserialize)]
struct RawClassCertification {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "AircraftClassId", default)]
    aircraft_class_id: Option<Uuid>,
    #[serde(rename = "AircraftClass", default)]
    aircraft_class: Option<RawAircraftClass>,
    #[serde(
        rename = "LastValidation",
        default,
        deserialize_with = "deserialize_onair_datetime"
    )]
    last_validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct RawAircraftClass {
    #[serde(rename = "Id", default)]
    id: Option<Uuid>,
    #[serde(rename = "ShortName", default)]
    short_name: Option<String>,
    #[serde(rename = "Name", default)]
    name: Option<String>,
}

impl OnAirClient {
    pub fn new(
        base_url: &str,
        company_id: Uuid,
        api_key: SecretString,
    ) -> Result<Self, ClientError> {
        let mut parsed = Url::parse(base_url)?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path()));
        }

        Ok(Self {
            http: Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            base_url: parsed,
            company_id,
            api_key,
        })
    }

    pub async fn company_summary(&self) -> Result<CompanySummary, ClientError> {
        let response: ApiResult<RawCompany> =
            self.get(&format!("company/{}", self.company_id)).await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let company = response.content.ok_or(ClientError::MissingContent)?;
        Ok(CompanySummary {
            id: CompanyId(company.id),
            name: company.name,
            airline_code: company.airline_code,
        })
    }

    pub async fn fleet(&self) -> Result<Observed<Vec<AircraftSummary>>, ClientError> {
        let response: ApiResult<Vec<RawAircraft>> = self
            .get(&format!("company/{}/fleet", self.company_id))
            .await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let aircraft = response
            .content
            .ok_or(ClientError::MissingContent)?
            .into_iter()
            .filter_map(translate_aircraft)
            .collect();

        Ok(Observed {
            value: aircraft,
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: FLEET_PROVENANCE_SOURCE.to_owned(),
                observed_at: Utc::now(),
            },
        })
    }

    pub async fn fbos(&self) -> Result<Observed<Vec<FboSummary>>, ClientError> {
        let response: ApiResult<Vec<RawFbo>> = self
            .get(&format!("company/{}/fbos", self.company_id))
            .await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let fbos = response
            .content
            .ok_or(ClientError::MissingContent)?
            .into_iter()
            .filter_map(translate_fbo)
            .collect();

        Ok(Observed {
            value: fbos,
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: FBOS_PROVENANCE_SOURCE.to_owned(),
                observed_at: Utc::now(),
            },
        })
    }

    pub async fn pending_jobs(&self) -> Result<Observed<JobSnapshot>, ClientError> {
        let response: ApiResult<Vec<RawMission>> = self
            .get(&format!("company/{}/jobs/pending", self.company_id))
            .await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let jobs = response
            .content
            .ok_or(ClientError::MissingContent)?
            .into_iter()
            .filter_map(translate_job)
            .take(MAX_JOBS_PER_SNAPSHOT)
            .collect();
        let snapshot = JobSnapshot {
            schema_version: JOB_SNAPSHOT_SCHEMA_VERSION,
            jobs,
        };
        snapshot
            .validate()
            .map_err(|_| ClientError::MissingContent)?;

        Ok(Observed {
            value: snapshot,
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: JOBS_PROVENANCE_SOURCE.to_owned(),
                observed_at: Utc::now(),
            },
        })
    }

    pub async fn staff(&self) -> Result<Observed<StaffSnapshot>, ClientError> {
        let response: ApiResult<Vec<RawStaffMember>> = self
            .get(&format!("company/{}/employees", self.company_id))
            .await?;
        if response.error.is_some_and(|error| !error.trim().is_empty()) {
            return Err(ClientError::ApiRejected);
        }

        let snapshot = StaffSnapshot {
            schema_version: STAFF_SNAPSHOT_SCHEMA_VERSION,
            staff: response
                .content
                .ok_or(ClientError::MissingContent)?
                .into_iter()
                .filter_map(translate_staff_member)
                .take(MAX_STAFF_PER_SNAPSHOT)
                .collect(),
        };
        snapshot
            .validate()
            .map_err(|_| ClientError::MissingContent)?;

        Ok(Observed {
            value: snapshot,
            provenance: Provenance {
                kind: ProvenanceKind::OnAirFact,
                source: STAFF_PROVENANCE_SOURCE.to_owned(),
                observed_at: Utc::now(),
            },
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let mut response = self.http.execute(self.request(path)?).await?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(ClientError::AuthenticationRejected);
            }
            StatusCode::NOT_FOUND => return Err(ClientError::CompanyNotFound),
            StatusCode::TOO_MANY_REQUESTS => return Err(ClientError::RateLimited),
            status if !status.is_success() => return Err(ClientError::Http(status)),
            _ => {}
        }

        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::ResponseTooLarge);
        }
        if response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                let value = value.to_ascii_lowercase();
                !value.starts_with("application/json") && !value.starts_with("text/json")
            })
        {
            return Err(ClientError::InvalidContentType);
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(ClientError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_| ClientError::MalformedResponse)
    }

    fn request(&self, path: &str) -> Result<reqwest::Request, ClientError> {
        let url = self.base_url.join(path)?;
        let api_key = header::HeaderValue::from_str(self.api_key.expose_secret())?;
        let company_id = header::HeaderValue::from_str(&self.company_id.to_string())?;

        Ok(self
            .http
            .get(url)
            .header(API_KEY_HEADER, api_key)
            .header(COMPANY_ID_HEADER, company_id)
            .header(header::ACCEPT, "application/json")
            .build()?)
    }
}

fn translate_aircraft(aircraft: RawAircraft) -> Option<AircraftSummary> {
    let current_airport = aircraft.current_airport.and_then(translate_airport);
    let direct_location = coordinates(aircraft.latitude, aircraft.longitude);
    let airport_location = current_airport
        .as_ref()
        .and_then(|airport| airport.location);

    Some(AircraftSummary {
        id: AircraftId(aircraft.id?),
        registration: non_empty(aircraft.identifier),
        model: aircraft.aircraft_type.and_then(|aircraft_type| {
            non_empty(aircraft_type.display_name).or_else(|| non_empty(aircraft_type.type_name))
        }),
        location: direct_location.or(airport_location),
        current_airport,
    })
}

fn translate_airport(airport: RawAirport) -> Option<AirportSummary> {
    Some(AirportSummary {
        id: AirportId(airport.id?),
        icao: non_empty(airport.icao),
        name: non_empty(airport.name),
        location: coordinates(airport.latitude, airport.longitude),
    })
}

fn translate_fbo(fbo: RawFbo) -> Option<FboSummary> {
    let airport = match fbo.airport {
        Some(mut airport) => {
            airport.id = airport.id.or(fbo.airport_id);
            translate_airport(airport)
        }
        None => fbo.airport_id.map(|id| AirportSummary {
            id: AirportId(id),
            icao: None,
            name: None,
            location: None,
        }),
    };

    Some(FboSummary {
        id: FboId(fbo.id?),
        name: non_empty(fbo.name),
        airport,
    })
}

fn translate_job(mission: RawMission) -> Option<JobSummary> {
    let mut legs = mission
        .cargos
        .into_iter()
        .filter_map(translate_cargo)
        .chain(mission.charters.into_iter().filter_map(translate_charter))
        .collect::<Vec<_>>();
    legs.sort_by_key(|(source_index, _, _)| *source_index);
    let legs = legs
        .into_iter()
        .enumerate()
        .map(|(sequence, (_, _, mut leg))| {
            leg.sequence = sequence as u32;
            leg
        })
        .collect::<Vec<_>>();

    let mission_type = mission.mission_type.and_then(|mission_type| {
        bounded_text(mission_type.name, 120).or_else(|| bounded_text(mission_type.short_name, 120))
    });
    let job = JobSummary {
        id: JobId(mission.id?),
        mission_type,
        description: bounded_text(mission.description, 1_024),
        reported_pay: non_negative_finite(mission.pay),
        created_at: mission.creation_date,
        taken_at: mission.taken_date,
        expires_at: mission.expiration_date,
        legs,
    };
    job.validate().is_ok().then_some(job)
}

fn translate_cargo(cargo: RawCargo) -> Option<(u32, u8, JobLeg)> {
    Some((
        cargo.leg_index.unwrap_or(u32::MAX),
        0,
        JobLeg {
            id: JobLegId(cargo.id?),
            sequence: 0,
            kind: JobLegKind::Cargo,
            departure: cargo.departure_airport.and_then(translate_airport),
            destination: cargo.destination_airport.and_then(translate_airport),
            current_airport: cargo.current_airport.and_then(translate_airport),
            cargo_weight_lb: non_negative_finite(cargo.weight),
            passengers: None,
            distance_nm: non_negative_finite(cargo.distance),
            description: bounded_text(cargo.description, 512),
        },
    ))
}

fn translate_charter(charter: RawCharter) -> Option<(u32, u8, JobLeg)> {
    Some((
        charter.leg_index.unwrap_or(u32::MAX),
        1,
        JobLeg {
            id: JobLegId(charter.id?),
            sequence: 0,
            kind: JobLegKind::Passengers,
            departure: charter.departure_airport.and_then(translate_airport),
            destination: charter.destination_airport.and_then(translate_airport),
            current_airport: charter.current_airport.and_then(translate_airport),
            cargo_weight_lb: None,
            passengers: charter
                .passengers
                .and_then(|value| u32::try_from(value).ok()),
            distance_nm: non_negative_finite(charter.distance),
            description: bounded_text(charter.description, 512),
        },
    ))
}

fn translate_staff_member(member: RawStaffMember) -> Option<StaffMemberSummary> {
    let mut qualification_ids = HashSet::new();
    let mut aircraft_class_ids = HashSet::new();
    let staff = StaffMemberSummary {
        id: StaffMemberId(member.id?),
        display_name: bounded_text(member.display_name, 120),
        avatar_reference: bounded_text(member.avatar_reference, 255),
        category_code: member.category_code.filter(|code| (0..=4).contains(code)),
        status_code: member.status_code.filter(|code| (0..=11).contains(code)),
        current_airport: member.current_airport.and_then(translate_airport),
        home_airport: member.home_airport.and_then(translate_airport),
        busy_until: member.busy_until,
        is_online: member.is_online,
        class_qualifications: member
            .class_certifications
            .into_iter()
            .filter_map(translate_class_qualification)
            .filter(|qualification| {
                qualification_ids.insert(qualification.id.clone())
                    && aircraft_class_ids.insert(qualification.aircraft_class_id.clone())
            })
            .take(MAX_CLASS_QUALIFICATIONS_PER_STAFF_MEMBER)
            .collect(),
    };
    staff.validate().is_ok().then_some(staff)
}

fn translate_class_qualification(
    qualification: RawClassCertification,
) -> Option<AircraftClassQualification> {
    let aircraft_class = qualification.aircraft_class;
    let declared_aircraft_class_id = qualification.aircraft_class_id;
    let nested_aircraft_class_id = aircraft_class.as_ref().and_then(|class| class.id);
    if declared_aircraft_class_id
        .zip(nested_aircraft_class_id)
        .is_some_and(|(declared, nested)| declared != nested)
    {
        return None;
    }
    let aircraft_class_id = declared_aircraft_class_id.or(nested_aircraft_class_id)?;
    Some(AircraftClassQualification {
        id: StaffQualificationId(qualification.id?),
        aircraft_class_id: AircraftClassId(aircraft_class_id),
        short_name: aircraft_class
            .as_ref()
            .and_then(|class| bounded_text(class.short_name.clone(), 64)),
        name: aircraft_class.and_then(|class| bounded_text(class.name, 160)),
        last_validated_at: qualification.last_validated_at,
    })
}

fn coordinates(latitude: Option<f64>, longitude: Option<f64>) -> Option<Coordinates> {
    let coordinates = Coordinates {
        latitude: latitude?,
        longitude: longitude?,
    };
    coordinates.is_valid().then_some(coordinates)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn bounded_text(value: Option<String>, maximum: usize) -> Option<String> {
    non_empty(value).filter(|value| {
        value.len() <= maximum && !value.chars().any(|character| character.is_control())
    })
}

fn non_negative_finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_onair_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(Some(timestamp.with_timezone(&Utc)));
    }

    const NAIVE_ONAIR_FORMATS: [&str; 2] = ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"];
    for format in NAIVE_ONAIR_FORMATS {
        if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(value, format) {
            return Ok(Some(timestamp.and_utc()));
        }
    }

    Ok(None)
}

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;

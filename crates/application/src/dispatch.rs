use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use thiserror::Error;
use wyrmgrid_domain::{
    AircraftSummary, FlightPlanSnapshot, JobSummary, Mass, MassUnit, OperationalProvenance,
    ProvenanceKind, SnapshotFreshness, WeatherSnapshot,
};
use wyrmgrid_simbrief_api::{ClientError, SimBriefClient, UserReference, UserReferenceKind};
use wyrmgrid_weather_api::{AviationWeatherClient, ClientError as WeatherClientError};

use crate::{FleetSnapshotView, SnapshotAvailability};

pub const WEATHER_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
pub const WEATHER_REQUEST_COOLDOWN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimBriefReferenceKind {
    PilotId,
    Username,
}

impl From<SimBriefReferenceKind> for UserReferenceKind {
    fn from(value: SimBriefReferenceKind) -> Self {
        match value {
            SimBriefReferenceKind::PilotId => Self::PilotId,
            SimBriefReferenceKind::Username => Self::Username,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchAvailability {
    Empty,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPersistence {
    SessionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchFindingStatus {
    Match,
    Difference,
    Information,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchFindingCategory {
    AircraftIdentity,
    AircraftModel,
    AircraftPosition,
    Payload,
    Schedule,
    JobRoute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AircraftMatchBasis {
    Registration,
    ExactModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatchedFleetAircraft {
    pub basis: AircraftMatchBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_airport_icao: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DispatchFinding {
    pub category: DispatchFindingCategory,
    pub status: DispatchFindingStatus,
    pub message_key: &'static str,
    pub title: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onair_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DispatchComparison {
    pub fleet_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_observed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_aircraft: Option<MatchedFleetAircraft>,
    pub findings: Vec<DispatchFinding>,
    pub provenance: OperationalProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchWeatherAvailability {
    NotRequested,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchWeatherCacheState {
    None,
    Fresh,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchWeatherStatus {
    pub provider_available: bool,
    pub availability: DispatchWeatherAvailability,
    pub refreshing: bool,
    pub cache: DispatchWeatherCacheState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<WeatherSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchStatus {
    pub provider_available: bool,
    pub availability: DispatchAvailability,
    pub persistence: DispatchPersistence,
    pub importing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<FlightPlanSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<DispatchComparison>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_job: Option<DispatchJobSelection>,
    pub weather: DispatchWeatherStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchJobSelection {
    pub job: JobSummary,
    pub observed_at: DateTime<Utc>,
    pub availability: SnapshotAvailability,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    #[error("The SimBrief importer is unavailable in this application session.")]
    ProviderUnavailable,
    #[error("A SimBrief import is already in progress.")]
    ImportInProgress,
    #[error("Import a flight plan before requesting airport weather.")]
    WeatherNeedsPlan,
    #[error("The aviation weather provider is unavailable in this application session.")]
    WeatherProviderUnavailable,
    #[error("An airport weather refresh is already in progress.")]
    WeatherRefreshInProgress,
    #[error("Wait before requesting airport weather again.")]
    WeatherRefreshTooSoon,
    #[error("The local Dispatch state is unavailable.")]
    StateUnavailable,
    #[error(transparent)]
    Provider(#[from] ClientError),
    #[error(transparent)]
    WeatherProvider(#[from] WeatherClientError),
}

type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<FlightPlanSnapshot, ClientError>> + Send + 'a>>;
type WeatherProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<WeatherSnapshot, WeatherClientError>> + Send + 'a>>;

trait FlightPlanProvider: Send + Sync {
    fn fetch_latest<'a>(
        &'a self,
        kind: SimBriefReferenceKind,
        value: &'a str,
    ) -> ProviderFuture<'a>;
}

trait WeatherProvider: Send + Sync {
    fn fetch_airports<'a>(&'a self, stations: &'a [String]) -> WeatherProviderFuture<'a>;
}

impl FlightPlanProvider for SimBriefClient {
    fn fetch_latest<'a>(
        &'a self,
        kind: SimBriefReferenceKind,
        value: &'a str,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let reference = UserReference::parse(kind.into(), value)?;
            SimBriefClient::fetch_latest(self, &reference).await
        })
    }
}

impl WeatherProvider for AviationWeatherClient {
    fn fetch_airports<'a>(&'a self, stations: &'a [String]) -> WeatherProviderFuture<'a> {
        Box::pin(AviationWeatherClient::fetch_airports(self, stations))
    }
}

struct CachedWeather {
    stations: Vec<String>,
    fetched_at: Instant,
    snapshot: WeatherSnapshot,
}

struct DispatchInner {
    provider: Option<Arc<dyn FlightPlanProvider>>,
    weather_provider: Option<Arc<dyn WeatherProvider>>,
    snapshot: RwLock<Option<FlightPlanSnapshot>>,
    weather: RwLock<Option<CachedWeather>>,
    selected_job: RwLock<Option<DispatchJobSelection>>,
    weather_last_attempt: Mutex<Option<Instant>>,
    importing: AtomicBool,
    weather_refreshing: AtomicBool,
}

#[derive(Clone)]
pub struct DispatchSession {
    inner: Arc<DispatchInner>,
}

impl DispatchSession {
    pub fn with_default_provider() -> Self {
        let provider = SimBriefClient::new()
            .ok()
            .map(|provider| Arc::new(provider) as Arc<dyn FlightPlanProvider>);
        let weather_provider = AviationWeatherClient::new()
            .ok()
            .map(|provider| Arc::new(provider) as Arc<dyn WeatherProvider>);
        Self::with_providers(provider, weather_provider)
    }

    fn with_providers(
        provider: Option<Arc<dyn FlightPlanProvider>>,
        weather_provider: Option<Arc<dyn WeatherProvider>>,
    ) -> Self {
        Self {
            inner: Arc::new(DispatchInner {
                provider,
                weather_provider,
                snapshot: RwLock::new(None),
                weather: RwLock::new(None),
                selected_job: RwLock::new(None),
                weather_last_attempt: Mutex::new(None),
                importing: AtomicBool::new(false),
                weather_refreshing: AtomicBool::new(false),
            }),
        }
    }

    pub fn status(&self) -> Result<DispatchStatus, DispatchError> {
        self.briefing(None)
    }

    pub fn briefing(
        &self,
        fleet: Option<&FleetSnapshotView>,
    ) -> Result<DispatchStatus, DispatchError> {
        let snapshot = self
            .inner
            .snapshot
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?
            .clone();
        let selected_job = self
            .inner
            .selected_job
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?
            .clone();
        let comparison = snapshot
            .as_ref()
            .map(|snapshot| compare_plan_to_fleet(snapshot, fleet, selected_job.as_ref()));
        Ok(DispatchStatus {
            provider_available: self.inner.provider.is_some(),
            availability: if snapshot.is_some() {
                DispatchAvailability::Ready
            } else {
                DispatchAvailability::Empty
            },
            persistence: DispatchPersistence::SessionOnly,
            importing: self.inner.importing.load(Ordering::Acquire),
            snapshot,
            comparison,
            selected_job,
            weather: self.weather_status()?,
        })
    }

    pub async fn import_latest(
        &self,
        kind: SimBriefReferenceKind,
        value: &str,
    ) -> Result<DispatchStatus, DispatchError> {
        let provider = self
            .inner
            .provider
            .as_ref()
            .ok_or(DispatchError::ProviderUnavailable)?
            .clone();
        if self
            .inner
            .importing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(DispatchError::ImportInProgress);
        }
        let _guard = ActivityGuard(&self.inner.importing);
        let snapshot = provider.fetch_latest(kind, value).await?;
        snapshot
            .validate()
            .map_err(|_| DispatchError::Provider(ClientError::MalformedPlan))?;
        *self
            .inner
            .snapshot
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = Some(snapshot);
        self.clear_weather_state()?;
        self.status()
    }

    pub async fn refresh_weather(&self) -> Result<WeatherSnapshot, DispatchError> {
        let provider = self
            .inner
            .weather_provider
            .as_ref()
            .ok_or(DispatchError::WeatherProviderUnavailable)?
            .clone();
        let stations = self.weather_stations()?;
        if let Some(snapshot) = self.fresh_cached_weather(&stations)? {
            return Ok(snapshot);
        }
        if self
            .inner
            .weather_refreshing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(DispatchError::WeatherRefreshInProgress);
        }
        let _guard = ActivityGuard(&self.inner.weather_refreshing);
        let now = Instant::now();
        {
            let mut last_attempt = self
                .inner
                .weather_last_attempt
                .lock()
                .map_err(|_| DispatchError::StateUnavailable)?;
            if last_attempt
                .is_some_and(|previous| now.duration_since(previous) < WEATHER_REQUEST_COOLDOWN)
            {
                return Err(DispatchError::WeatherRefreshTooSoon);
            }
            *last_attempt = Some(now);
        }

        let snapshot = provider.fetch_airports(&stations).await?;
        snapshot
            .validate()
            .map_err(|_| DispatchError::WeatherProvider(WeatherClientError::MalformedWeather))?;
        *self
            .inner
            .weather
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = Some(CachedWeather {
            stations,
            fetched_at: Instant::now(),
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub fn clear(&self) -> Result<DispatchStatus, DispatchError> {
        if self.inner.importing.load(Ordering::Acquire) {
            return Err(DispatchError::ImportInProgress);
        }
        *self
            .inner
            .snapshot
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = None;
        self.clear_weather_state()?;
        self.status()
    }

    pub fn select_job(&self, selection: DispatchJobSelection) -> Result<(), DispatchError> {
        selection
            .job
            .validate()
            .map_err(|_| DispatchError::StateUnavailable)?;
        *self
            .inner
            .selected_job
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = Some(selection);
        Ok(())
    }

    pub fn clear_job(&self) -> Result<(), DispatchError> {
        *self
            .inner
            .selected_job
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = None;
        Ok(())
    }

    fn weather_stations(&self) -> Result<Vec<String>, DispatchError> {
        let snapshot = self
            .inner
            .snapshot
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?;
        let airports = snapshot
            .as_ref()
            .ok_or(DispatchError::WeatherNeedsPlan)?
            .airports
            .value
            .clone();
        let mut stations = vec![airports.origin.icao, airports.destination.icao];
        stations.extend(airports.alternates.into_iter().map(|airport| airport.icao));
        stations.sort();
        stations.dedup();
        Ok(stations)
    }

    fn fresh_cached_weather(
        &self,
        stations: &[String],
    ) -> Result<Option<WeatherSnapshot>, DispatchError> {
        Ok(self
            .inner
            .weather
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?
            .as_ref()
            .filter(|cached| {
                cached.stations == stations && cached.fetched_at.elapsed() < WEATHER_CACHE_TTL
            })
            .map(|cached| cached.snapshot.clone()))
    }

    fn weather_status(&self) -> Result<DispatchWeatherStatus, DispatchError> {
        let cache = self
            .inner
            .weather
            .read()
            .map_err(|_| DispatchError::StateUnavailable)?;
        let (cache_state, snapshot) =
            cache
                .as_ref()
                .map_or((DispatchWeatherCacheState::None, None), |cached| {
                    let state = if cached.fetched_at.elapsed() < WEATHER_CACHE_TTL {
                        DispatchWeatherCacheState::Fresh
                    } else {
                        DispatchWeatherCacheState::Expired
                    };
                    (state, Some(cached.snapshot.clone()))
                });
        Ok(DispatchWeatherStatus {
            provider_available: self.inner.weather_provider.is_some(),
            availability: if snapshot.is_some() {
                DispatchWeatherAvailability::Ready
            } else {
                DispatchWeatherAvailability::NotRequested
            },
            refreshing: self.inner.weather_refreshing.load(Ordering::Acquire),
            cache: cache_state,
            snapshot,
        })
    }

    fn clear_weather_state(&self) -> Result<(), DispatchError> {
        *self
            .inner
            .weather
            .write()
            .map_err(|_| DispatchError::StateUnavailable)? = None;
        *self
            .inner
            .weather_last_attempt
            .lock()
            .map_err(|_| DispatchError::StateUnavailable)? = None;
        Ok(())
    }
}

struct ActivityGuard<'a>(&'a AtomicBool);

impl Drop for ActivityGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn compare_plan_to_fleet(
    plan: &FlightPlanSnapshot,
    fleet: Option<&FleetSnapshotView>,
    selected_job: Option<&DispatchJobSelection>,
) -> DispatchComparison {
    let compared_at = Utc::now();
    let fleet_observed_at = fleet.map(|fleet| fleet.snapshot.provenance.observed_at);
    let freshness = match fleet.map(|fleet| fleet.availability) {
        Some(SnapshotAvailability::Offline) => SnapshotFreshness::Stale,
        Some(_) => SnapshotFreshness::Current,
        None => SnapshotFreshness::Unknown,
    };
    let mut findings = Vec::new();
    let matched =
        fleet.and_then(|fleet| match_aircraft(plan, &fleet.snapshot.value, &mut findings));

    if fleet.is_none() {
        findings.push(finding(
            DispatchFindingCategory::AircraftIdentity,
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-onair-fleet-unavailable",
            "OnAir fleet unavailable",
            "Connect or synchronize OnAir to compare this plan with an observed company aircraft.",
            plan_aircraft_label(plan),
            None,
        ));
    }
    append_model_finding(plan, matched.map(|(aircraft, _)| aircraft), &mut findings);
    append_position_finding(plan, matched.map(|(aircraft, _)| aircraft), &mut findings);
    append_job_findings(
        plan,
        selected_job.map(|selection| &selection.job),
        &mut findings,
    );

    let matched_aircraft = matched.map(|(aircraft, basis)| MatchedFleetAircraft {
        basis,
        registration: aircraft.registration.clone(),
        model: aircraft.model.clone(),
        current_airport_icao: aircraft
            .current_airport
            .as_ref()
            .and_then(|airport| airport.icao.clone()),
    });
    DispatchComparison {
        fleet_available: fleet.is_some(),
        fleet_observed_at,
        matched_aircraft,
        findings,
        provenance: OperationalProvenance {
            kind: ProvenanceKind::Calculated,
            provider: "wyrmgrid".into(),
            provider_revision: None,
            generated_at: Some(compared_at),
            retrieved_at: compared_at,
            transformation_version: 1,
            freshness,
        },
    }
}

fn append_job_findings(
    plan: &FlightPlanSnapshot,
    job: Option<&JobSummary>,
    findings: &mut Vec<DispatchFinding>,
) {
    let Some(job) = job else {
        findings.push(finding(
            DispatchFindingCategory::JobRoute,
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-job-unselected",
            "No OnAir job selected",
            "Choose a pending job to compare its route, payload, and expiry with this plan.",
            Some(format!(
                "{} → {}",
                plan.airports.value.origin.icao, plan.airports.value.destination.icao
            )),
            None,
        ));
        return;
    };

    let planned_route = format!(
        "{} → {}",
        plan.airports.value.origin.icao, plan.airports.value.destination.icao
    );
    let job_route = job.route().and_then(|(departure, destination)| {
        Some(format!(
            "{} → {}",
            departure.icao.as_deref()?,
            destination.icao.as_deref()?
        ))
    });
    let route_matches = job_route
        .as_ref()
        .is_some_and(|route| normalize_identity(route) == normalize_identity(&planned_route));
    let (route_status, route_key, route_title, route_explanation) = if route_matches {
        (
            DispatchFindingStatus::Match,
            "dispatch-finding-job-route-match",
            "Job route matched",
            "The SimBrief endpoints match the first departure and final destination in the selected job.",
        )
    } else if job_route.is_some() {
        (
            DispatchFindingStatus::Difference,
            "dispatch-finding-job-route-difference",
            "Job route differs",
            "The plan endpoints do not match the selected job.",
        )
    } else {
        (
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-job-route-unavailable",
            "Job route unavailable",
            "The selected job did not report both endpoint ICAOs.",
        )
    };
    findings.push(finding(
        DispatchFindingCategory::JobRoute,
        route_status,
        route_key,
        route_title,
        route_explanation,
        Some(planned_route),
        job_route,
    ));

    let planned_payload = plan
        .weights
        .as_ref()
        .and_then(|weights| weights.value.payload);
    let job_payload = job.cargo_weight_lb();
    let payload_matches = planned_payload.zip(job_payload).map(|(plan, job)| {
        let plan_lb = match plan.unit {
            MassUnit::Pounds => plan.value,
            MassUnit::Kilograms => plan.value * 2.204_622_621_8,
        };
        (plan_lb - job).abs() <= 1.0
    });
    findings.push(finding(
        DispatchFindingCategory::Payload,
        match payload_matches {
            Some(true) => DispatchFindingStatus::Match,
            Some(false) => DispatchFindingStatus::Difference,
            None => DispatchFindingStatus::Unavailable,
        },
        match payload_matches {
            Some(true) => "dispatch-finding-job-payload-match",
            Some(false) => "dispatch-finding-job-payload-difference",
            None => "dispatch-finding-job-payload-unavailable",
        },
        match payload_matches {
            Some(true) => "Job payload matched",
            Some(false) => "Job payload differs",
            None => "Cargo comparison unavailable",
        },
        "WyrmGrid compares reported SimBrief payload with the selected job's cargo weight; it does not infer aircraft capacity.",
        planned_payload.map(format_mass),
        job_payload.map(|value| format!("{value} lb")),
    ));

    let planned_arrival = plan
        .schedule
        .as_ref()
        .and_then(|schedule| schedule.value.scheduled_in.or(schedule.value.scheduled_on));
    let deadline_match = planned_arrival
        .zip(job.expires_at)
        .map(|(arrival, expiry)| arrival <= expiry);
    findings.push(finding(
        DispatchFindingCategory::Schedule,
        match deadline_match {
            Some(true) => DispatchFindingStatus::Match,
            Some(false) => DispatchFindingStatus::Difference,
            None => DispatchFindingStatus::Unavailable,
        },
        match deadline_match {
            Some(true) => "dispatch-finding-job-deadline-match",
            Some(false) => "dispatch-finding-job-deadline-missed",
            None => "dispatch-finding-job-deadline-unavailable",
        },
        match deadline_match {
            Some(true) => "Planned arrival precedes expiry",
            Some(false) => "Planned arrival follows expiry",
            None => "Deadline comparison unavailable",
        },
        "This is a direct schedule comparison, not a guarantee that OnAir will accept or complete the job.",
        planned_arrival.map(|value| value.to_rfc3339()),
        job.expires_at.map(|value| value.to_rfc3339()),
    ));
}

fn format_mass(mass: Mass) -> String {
    let unit = match mass.unit {
        MassUnit::Kilograms => "kg",
        MassUnit::Pounds => "lb",
    };
    format!("{} {unit}", mass.value)
}

fn match_aircraft<'a>(
    plan: &FlightPlanSnapshot,
    fleet: &'a [AircraftSummary],
    findings: &mut Vec<DispatchFinding>,
) -> Option<(&'a AircraftSummary, AircraftMatchBasis)> {
    let planned = plan.aircraft.as_ref().map(|aircraft| &aircraft.value);
    if let Some(registration) = planned.and_then(|aircraft| aircraft.registration.as_deref()) {
        let matches = fleet
            .iter()
            .filter(|aircraft| {
                aircraft.registration.as_deref().is_some_and(|value| {
                    normalize_identity(value) == normalize_identity(registration)
                })
            })
            .collect::<Vec<_>>();
        return match matches.as_slice() {
            [aircraft] => {
                findings.push(finding(
                    DispatchFindingCategory::AircraftIdentity,
                    DispatchFindingStatus::Match,
                    "dispatch-finding-registration-match",
                    "Registration matched",
                    "The SimBrief registration exactly matches one aircraft in the observed OnAir fleet.",
                    Some(registration.into()),
                    aircraft.registration.clone(),
                ));
                Some((aircraft, AircraftMatchBasis::Registration))
            }
            [] => {
                findings.push(finding(
                    DispatchFindingCategory::AircraftIdentity,
                    DispatchFindingStatus::Difference,
                    "dispatch-finding-registration-not-found",
                    "Registration not found",
                    "No aircraft in the observed OnAir fleet has the SimBrief registration.",
                    Some(registration.into()),
                    None,
                ));
                None
            }
            _ => {
                findings.push(finding(
                    DispatchFindingCategory::AircraftIdentity,
                    DispatchFindingStatus::Unavailable,
                    "dispatch-finding-registration-ambiguous",
                    "Registration is ambiguous",
                    "More than one observed aircraft uses this registration, so WyrmGrid will not choose one.",
                    Some(registration.into()),
                    Some(format!("{} matches", matches.len())),
                ));
                None
            }
        };
    }

    if let Some(model) = planned.and_then(|aircraft| aircraft.model.as_deref()) {
        let matches = fleet
            .iter()
            .filter(|aircraft| {
                aircraft
                    .model
                    .as_deref()
                    .is_some_and(|value| normalize_identity(value) == normalize_identity(model))
            })
            .collect::<Vec<_>>();
        if let [aircraft] = matches.as_slice() {
            findings.push(finding(
                DispatchFindingCategory::AircraftIdentity,
                DispatchFindingStatus::Information,
                "dispatch-finding-model-candidate",
                "Unique model candidate",
                "One OnAir aircraft has the exact model label, but a model match does not prove it is the same airframe.",
                Some(model.into()),
                aircraft.registration.clone(),
            ));
            return Some((aircraft, AircraftMatchBasis::ExactModel));
        }
        findings.push(finding(
            DispatchFindingCategory::AircraftIdentity,
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-airframe-unmatched",
            "No deterministic airframe match",
            "The plan has no registration and its model label does not identify exactly one OnAir aircraft.",
            Some(model.into()),
            Some(format!("{} exact model candidates", matches.len())),
        ));
        return None;
    }

    findings.push(finding(
        DispatchFindingCategory::AircraftIdentity,
        DispatchFindingStatus::Unavailable,
        "dispatch-finding-plan-aircraft-missing",
        "Plan aircraft identity missing",
        "SimBrief supplied neither a registration nor a model that can be compared deterministically.",
        plan_aircraft_label(plan),
        None,
    ));
    None
}

fn append_model_finding(
    plan: &FlightPlanSnapshot,
    matched: Option<&AircraftSummary>,
    findings: &mut Vec<DispatchFinding>,
) {
    let planned = plan.aircraft.as_ref().map(|aircraft| &aircraft.value);
    let plan_model = planned.and_then(|aircraft| aircraft.model.clone());
    let plan_type = planned.and_then(|aircraft| aircraft.icao_type.clone());
    let onair_model = matched.and_then(|aircraft| aircraft.model.clone());
    let (status, message_key, title, explanation) = match (&plan_model, &onair_model) {
        (Some(plan_model), Some(onair_model))
            if normalize_identity(plan_model) == normalize_identity(onair_model) =>
        {
            (
                DispatchFindingStatus::Match,
                "dispatch-finding-model-match",
                "Model label matched",
                "SimBrief and OnAir supplied the same normalized model label.",
            )
        }
        (Some(_), Some(_)) => (
            DispatchFindingStatus::Difference,
            "dispatch-finding-model-difference",
            "Model labels differ",
            "The source labels differ. WyrmGrid has not used a type catalogue to declare them compatible or incompatible.",
        ),
        (_, Some(_)) if plan_type.is_some() => (
            DispatchFindingStatus::Information,
            "dispatch-finding-aircraft-vocabularies",
            "Different aircraft vocabularies",
            "SimBrief supplied an ICAO type while OnAir supplied a model label; no unverified crosswalk was applied.",
        ),
        _ => (
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-model-unavailable",
            "Model comparison unavailable",
            "One or both sources did not provide comparable aircraft model text.",
        ),
    };
    findings.push(finding(
        DispatchFindingCategory::AircraftModel,
        status,
        message_key,
        title,
        explanation,
        plan_model.or(plan_type),
        onair_model,
    ));
}

fn append_position_finding(
    plan: &FlightPlanSnapshot,
    matched: Option<&AircraftSummary>,
    findings: &mut Vec<DispatchFinding>,
) {
    let origin = plan.airports.value.origin.icao.clone();
    let current_airport = matched
        .and_then(|aircraft| aircraft.current_airport.as_ref())
        .and_then(|airport| airport.icao.clone());
    let (status, message_key, title, explanation) = match current_airport.as_deref() {
        Some(value) if value.eq_ignore_ascii_case(&origin) => (
            DispatchFindingStatus::Match,
            "dispatch-finding-position-match",
            "Aircraft positioned at origin",
            "The matched OnAir aircraft is currently observed at the planned departure airport.",
        ),
        Some(_) => (
            DispatchFindingStatus::Difference,
            "dispatch-finding-position-difference",
            "Aircraft is away from origin",
            "The matched OnAir aircraft is observed at a different airport; positioning may be required.",
        ),
        None => (
            DispatchFindingStatus::Unavailable,
            "dispatch-finding-position-unavailable",
            "Position comparison unavailable",
            "No deterministically matched aircraft with a current OnAir airport is available.",
        ),
    };
    findings.push(finding(
        DispatchFindingCategory::AircraftPosition,
        status,
        message_key,
        title,
        explanation,
        Some(origin),
        current_airport,
    ));
}

fn finding(
    category: DispatchFindingCategory,
    status: DispatchFindingStatus,
    message_key: &'static str,
    title: &str,
    explanation: &str,
    plan_value: Option<String>,
    onair_value: Option<String>,
) -> DispatchFinding {
    DispatchFinding {
        category,
        status,
        message_key,
        title: title.into(),
        explanation: explanation.into(),
        plan_value,
        onair_value,
    }
}

fn plan_aircraft_label(plan: &FlightPlanSnapshot) -> Option<String> {
    let aircraft = plan.aircraft.as_ref().map(|aircraft| &aircraft.value)?;
    aircraft
        .registration
        .clone()
        .or_else(|| aircraft.model.clone())
        .or_else(|| aircraft.icao_type.clone())
}

fn normalize_identity(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
#[path = "tests/dispatch.rs"]
mod tests;

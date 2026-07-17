use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{
    FLIGHT_OPERATION_SCHEMA_VERSION, FlightManifest, FlightOperationId, FlightOperationRevision,
    FlightOperationRevisionReason, OperationalObservation, OperationalProvenance, ProvenanceKind,
    SnapshotFreshness,
};
use wyrmgrid_storage::{FlightOperationRevisionRecord, Store};

use crate::{DispatchJobSelection, DispatchStatus, SnapshotAvailability};

pub const FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationStage {
    Plan,
    Weather,
    Jobs,
    Manifest,
    Fleet,
    Staff,
    Review,
    Atlas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationStageState {
    NotStarted,
    Available,
    Ready,
    NeedsAttention,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FlightOperationStageView {
    pub stage: FlightOperationStage,
    pub state: FlightOperationStageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightOperationJourneyView {
    pub schema_version: u32,
    pub stages: Vec<FlightOperationStageView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightOperationContextChange {
    None,
    Plan,
    Job,
    PlanAndJob,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightOperationView {
    pub schema_version: u32,
    pub id: String,
    pub revision: u32,
    pub reason: FlightOperationRevisionReason,
    pub operation_created_at: DateTime<Utc>,
    pub revision_created_at: DateTime<Utc>,
    pub plan_id: String,
    pub origin: String,
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_job_id: Option<String>,
    pub manifest: FlightManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightOperationAvailability {
    pub jobs: bool,
    pub fleet: bool,
    pub staff: bool,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FlightOperationError {
    #[error("Import a flight plan before starting an operation.")]
    PlanRequired,
    #[error("A flight operation is already active.")]
    ActiveOperationExists,
    #[error("There is no active flight operation to revise.")]
    NoActiveOperation,
    #[error("The Dispatch context has not changed since the current revision.")]
    NoRevisionChange,
    #[error("The saved flight operation is invalid or unsupported.")]
    InvalidStoredOperation,
    #[error("The flight-operation store is unavailable.")]
    StorageUnavailable,
}

#[derive(Clone)]
pub struct FlightOperationService {
    store: Store,
}

impl FlightOperationService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn enrich_dispatch_status(
        &self,
        status: &mut DispatchStatus,
        availability: FlightOperationAvailability,
    ) -> Result<(), FlightOperationError> {
        let revision = self.load_active_revision()?;
        status.operation_change = revision
            .as_ref()
            .map_or(FlightOperationContextChange::None, |revision| {
                context_change(revision, status)
            });
        status.operation = revision.as_ref().map(operation_view);

        let operation_matches_plan = revision.as_ref().is_some_and(|revision| {
            status
                .snapshot
                .as_ref()
                .is_some_and(|plan| plan == &revision.plan)
        });
        let manifest = revision.as_ref().map(|revision| &revision.manifest);
        status.journey = build_flight_operation_journey(LifecycleJourneyEvidence {
            plan_provider_available: status.provider_available,
            plan_available: status.snapshot.is_some() || revision.is_some(),
            plan_stale: revision.is_some() && status.snapshot.is_some() && !operation_matches_plan,
            weather_provider_available: status.weather.provider_available,
            weather_available: operation_matches_plan && status.weather.snapshot.is_some(),
            weather_stale: operation_matches_plan
                && status.weather.cache == crate::DispatchWeatherCacheState::Expired,
            jobs_available: availability.jobs,
            job_selected: revision
                .as_ref()
                .is_some_and(|revision| revision.selected_job.is_some()),
            operation_available: revision.is_some(),
            manifest_available: manifest.is_some_and(|manifest| !manifest.legs.is_empty()),
            manifest_needs_attention: manifest.is_some_and(FlightManifest::needs_attention),
            fleet_available: availability.fleet,
            staff_available: availability.staff,
            atlas_available: status.atlas_plan.is_some(),
        });
        Ok(())
    }

    pub fn start_from_dispatch(&self, status: &DispatchStatus) -> Result<(), FlightOperationError> {
        if self.load_active_revision()?.is_some() {
            return Err(FlightOperationError::ActiveOperationExists);
        }
        let plan = status
            .snapshot
            .clone()
            .ok_or(FlightOperationError::PlanRequired)?;
        let now = Utc::now();
        let operation_id = FlightOperationId(Uuid::new_v4());
        let selected_job = status.selected_job.as_ref().map(job_observation);
        let manifest = selected_job
            .as_ref()
            .map_or_else(FlightManifest::empty, |job| {
                FlightManifest::from_job(&job.value)
            });
        let revision = FlightOperationRevision {
            schema_version: FLIGHT_OPERATION_SCHEMA_VERSION,
            operation_id,
            revision: 1,
            reason: FlightOperationRevisionReason::Initial,
            operation_created_at: now,
            revised_at: now,
            plan,
            selected_job_company_id: status
                .selected_job
                .as_ref()
                .map(|selection| selection.company_id.clone()),
            selected_job,
            manifest,
        };
        self.save_new_revision(&revision)
    }

    pub fn revise_from_dispatch(
        &self,
        status: &DispatchStatus,
    ) -> Result<(), FlightOperationError> {
        let current = self
            .load_active_revision()?
            .ok_or(FlightOperationError::NoActiveOperation)?;
        let change = context_change(&current, status);
        if change == FlightOperationContextChange::None {
            return Err(FlightOperationError::NoRevisionChange);
        }
        let plan = status
            .snapshot
            .clone()
            .ok_or(FlightOperationError::PlanRequired)?;
        let selected_job = status.selected_job.as_ref().map(job_observation);
        let manifest = selected_job
            .as_ref()
            .map_or_else(FlightManifest::empty, |job| {
                FlightManifest::from_job(&job.value)
            });
        let reason = match change {
            FlightOperationContextChange::Plan => FlightOperationRevisionReason::PlanChanged,
            FlightOperationContextChange::Job => FlightOperationRevisionReason::JobChanged,
            FlightOperationContextChange::PlanAndJob => {
                FlightOperationRevisionReason::PlanAndJobChanged
            }
            FlightOperationContextChange::None => unreachable!(),
        };
        let now = std::cmp::max(Utc::now(), current.revised_at);
        let revision = FlightOperationRevision {
            schema_version: FLIGHT_OPERATION_SCHEMA_VERSION,
            operation_id: current.operation_id.clone(),
            revision: current.revision.saturating_add(1),
            reason,
            operation_created_at: current.operation_created_at,
            revised_at: now,
            plan,
            selected_job_company_id: status
                .selected_job
                .as_ref()
                .map(|selection| selection.company_id.clone()),
            selected_job,
            manifest,
        };
        self.append_revision(&revision, current.revision)
    }

    fn load_active_revision(
        &self,
    ) -> Result<Option<FlightOperationRevision>, FlightOperationError> {
        let Some(record) = self
            .store
            .load_active_flight_operation_revision_record()
            .map_err(|_| FlightOperationError::StorageUnavailable)?
        else {
            return Ok(None);
        };
        let revision: FlightOperationRevision = serde_json::from_str(&record.snapshot_json)
            .map_err(|_| FlightOperationError::InvalidStoredOperation)?;
        revision
            .validate()
            .map_err(|_| FlightOperationError::InvalidStoredOperation)?;
        if revision.operation_id.0.to_string() != record.operation_id
            || revision.revision != record.revision
            || revision_reason_name(revision.reason) != record.reason
            || revision.operation_created_at.to_rfc3339() != record.operation_created_at
            || revision.revised_at.to_rfc3339() != record.revision_created_at
        {
            return Err(FlightOperationError::InvalidStoredOperation);
        }
        Ok(Some(revision))
    }

    fn save_new_revision(
        &self,
        revision: &FlightOperationRevision,
    ) -> Result<(), FlightOperationError> {
        revision
            .validate()
            .map_err(|_| FlightOperationError::InvalidStoredOperation)?;
        let record = storage_record(revision)?;
        self.store
            .create_flight_operation_record(&record)
            .map_err(|_| FlightOperationError::StorageUnavailable)
    }

    fn append_revision(
        &self,
        revision: &FlightOperationRevision,
        expected_revision: u32,
    ) -> Result<(), FlightOperationError> {
        revision
            .validate()
            .map_err(|_| FlightOperationError::InvalidStoredOperation)?;
        let record = storage_record(revision)?;
        self.store
            .append_flight_operation_revision_record(expected_revision, &record)
            .map_err(|_| FlightOperationError::StorageUnavailable)
    }
}

pub(crate) struct InitialJourneyEvidence {
    pub plan_provider_available: bool,
    pub plan_available: bool,
    pub weather_provider_available: bool,
    pub weather_available: bool,
    pub weather_stale: bool,
    pub atlas_available: bool,
}

struct LifecycleJourneyEvidence {
    plan_provider_available: bool,
    plan_available: bool,
    plan_stale: bool,
    weather_provider_available: bool,
    weather_available: bool,
    weather_stale: bool,
    jobs_available: bool,
    job_selected: bool,
    operation_available: bool,
    manifest_available: bool,
    manifest_needs_attention: bool,
    fleet_available: bool,
    staff_available: bool,
    atlas_available: bool,
}

pub(crate) fn build_initial_flight_operation_journey(
    evidence: InitialJourneyEvidence,
) -> FlightOperationJourneyView {
    build_flight_operation_journey(LifecycleJourneyEvidence {
        plan_provider_available: evidence.plan_provider_available,
        plan_available: evidence.plan_available,
        plan_stale: false,
        weather_provider_available: evidence.weather_provider_available,
        weather_available: evidence.weather_available,
        weather_stale: evidence.weather_stale,
        jobs_available: false,
        job_selected: false,
        operation_available: false,
        manifest_available: false,
        manifest_needs_attention: false,
        fleet_available: false,
        staff_available: false,
        atlas_available: evidence.atlas_available,
    })
}

fn build_flight_operation_journey(
    evidence: LifecycleJourneyEvidence,
) -> FlightOperationJourneyView {
    let plan = if evidence.plan_available && evidence.plan_stale {
        FlightOperationStageState::Stale
    } else if evidence.plan_available {
        FlightOperationStageState::Ready
    } else if evidence.plan_provider_available {
        FlightOperationStageState::Available
    } else {
        FlightOperationStageState::Unavailable
    };
    let weather = if !evidence.plan_available || !evidence.weather_provider_available {
        FlightOperationStageState::Unavailable
    } else if evidence.weather_available && evidence.weather_stale {
        FlightOperationStageState::Stale
    } else if evidence.weather_available {
        FlightOperationStageState::Ready
    } else {
        FlightOperationStageState::Available
    };
    let atlas = if evidence.atlas_available {
        FlightOperationStageState::Ready
    } else {
        FlightOperationStageState::Unavailable
    };
    let jobs = if evidence.job_selected {
        FlightOperationStageState::Ready
    } else if evidence.jobs_available {
        FlightOperationStageState::Available
    } else if evidence.operation_available {
        FlightOperationStageState::Unavailable
    } else {
        FlightOperationStageState::NotStarted
    };
    let manifest = if !evidence.operation_available {
        FlightOperationStageState::NotStarted
    } else if evidence.manifest_available && evidence.manifest_needs_attention {
        FlightOperationStageState::NeedsAttention
    } else if evidence.manifest_available {
        FlightOperationStageState::Ready
    } else {
        FlightOperationStageState::Available
    };
    let fleet = if !evidence.operation_available {
        FlightOperationStageState::NotStarted
    } else if evidence.fleet_available {
        FlightOperationStageState::Available
    } else {
        FlightOperationStageState::Unavailable
    };
    let staff = if !evidence.operation_available {
        FlightOperationStageState::NotStarted
    } else if evidence.staff_available {
        FlightOperationStageState::Available
    } else {
        FlightOperationStageState::Unavailable
    };
    let review = if evidence.operation_available {
        FlightOperationStageState::Available
    } else {
        FlightOperationStageState::NotStarted
    };

    FlightOperationJourneyView {
        schema_version: FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION,
        stages: vec![
            stage(FlightOperationStage::Plan, plan),
            stage(FlightOperationStage::Weather, weather),
            stage(FlightOperationStage::Jobs, jobs),
            stage(FlightOperationStage::Manifest, manifest),
            stage(FlightOperationStage::Fleet, fleet),
            stage(FlightOperationStage::Staff, staff),
            stage(FlightOperationStage::Review, review),
            stage(FlightOperationStage::Atlas, atlas),
        ],
    }
}

fn job_observation(
    selection: &DispatchJobSelection,
) -> OperationalObservation<wyrmgrid_domain::JobSummary> {
    OperationalObservation {
        value: selection.job.clone(),
        provenance: OperationalProvenance {
            kind: ProvenanceKind::OnAirFact,
            provider: "OnAir".into(),
            provider_revision: None,
            generated_at: None,
            retrieved_at: selection.observed_at,
            transformation_version: wyrmgrid_domain::JOB_SNAPSHOT_SCHEMA_VERSION,
            freshness: match selection.availability {
                SnapshotAvailability::Live => SnapshotFreshness::Current,
                SnapshotAvailability::Cached | SnapshotAvailability::Offline => {
                    SnapshotFreshness::Stale
                }
            },
        },
    }
}

fn context_change(
    current: &FlightOperationRevision,
    status: &DispatchStatus,
) -> FlightOperationContextChange {
    let plan_changed = status
        .snapshot
        .as_ref()
        .is_some_and(|plan| plan != &current.plan);
    let current_job = current
        .selected_job
        .as_ref()
        .zip(current.selected_job_company_id.as_ref())
        .map(|(job, company_id)| (&job.value, company_id));
    let dispatch_job = status
        .selected_job
        .as_ref()
        .map(|selection| (&selection.job, &selection.company_id));
    // An empty session after restart is not an instruction to detach retained
    // job evidence. A newly selected job is the evidence that can request a
    // reviewed revision; detachment needs its own explicit operation action.
    let job_changed = dispatch_job.is_some() && current_job != dispatch_job;
    match (plan_changed, job_changed) {
        (false, false) => FlightOperationContextChange::None,
        (true, false) => FlightOperationContextChange::Plan,
        (false, true) => FlightOperationContextChange::Job,
        (true, true) => FlightOperationContextChange::PlanAndJob,
    }
}

fn operation_view(revision: &FlightOperationRevision) -> FlightOperationView {
    FlightOperationView {
        schema_version: revision.schema_version,
        id: revision.operation_id.0.to_string(),
        revision: revision.revision,
        reason: revision.reason,
        operation_created_at: revision.operation_created_at,
        revision_created_at: revision.revised_at,
        plan_id: revision.plan.id.0.to_string(),
        origin: revision.plan.airports.value.origin.icao.clone(),
        destination: revision.plan.airports.value.destination.icao.clone(),
        selected_job_id: revision
            .selected_job
            .as_ref()
            .map(|job| job.value.id.0.to_string()),
        manifest: revision.manifest.clone(),
    }
}

fn storage_record(
    revision: &FlightOperationRevision,
) -> Result<FlightOperationRevisionRecord, FlightOperationError> {
    Ok(FlightOperationRevisionRecord {
        operation_id: revision.operation_id.0.to_string(),
        operation_created_at: revision.operation_created_at.to_rfc3339(),
        revision: revision.revision,
        reason: revision_reason_name(revision.reason).into(),
        revision_created_at: revision.revised_at.to_rfc3339(),
        snapshot_json: serde_json::to_string(revision)
            .map_err(|_| FlightOperationError::InvalidStoredOperation)?,
    })
}

fn revision_reason_name(reason: FlightOperationRevisionReason) -> &'static str {
    match reason {
        FlightOperationRevisionReason::Initial => "initial",
        FlightOperationRevisionReason::PlanChanged => "plan_changed",
        FlightOperationRevisionReason::JobChanged => "job_changed",
        FlightOperationRevisionReason::PlanAndJobChanged => "plan_and_job_changed",
    }
}

fn stage(
    stage: FlightOperationStage,
    state: FlightOperationStageState,
) -> FlightOperationStageView {
    FlightOperationStageView { stage, state }
}

#[cfg(test)]
#[path = "tests/flight_operation.rs"]
mod tests;

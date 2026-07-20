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

use crate::{
    DispatchFinding, DispatchFindingCategory, DispatchFindingStatus, DispatchJobSelection,
    DispatchStatus, FleetSnapshotView, MatchedFleetAircraft, SnapshotAvailability,
};

pub const FLIGHT_OPERATION_JOURNEY_SCHEMA_VERSION: u32 = 1;
pub const FLEET_RECONCILIATION_SCHEMA_VERSION: u32 = 1;

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
    pub fleet_reconciliation: FlightOperationFleetReconciliationView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightOperationManifestCoverageView {
    pub leg_count: usize,
    pub passenger_legs_reported: usize,
    pub freight_legs_reported: usize,
    pub source_gaps_present: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightOperationFleetReconciliationView {
    pub schema_version: u32,
    pub fleet_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_observed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<MatchedFleetAircraft>,
    pub manifest_coverage: FlightOperationManifestCoverageView,
    pub findings: Vec<DispatchFinding>,
    pub provenance: OperationalProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlightOperationAvailability {
    pub jobs: bool,
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
        fleet: Option<&FleetSnapshotView>,
    ) -> Result<(), FlightOperationError> {
        let revision = self.load_active_revision()?;
        status.operation_change = revision
            .as_ref()
            .map_or(FlightOperationContextChange::None, |revision| {
                context_change(revision, status)
            });
        let fleet_reconciliation = revision
            .as_ref()
            .map(|revision| build_fleet_reconciliation(revision, fleet));
        status.operation = revision
            .as_ref()
            .zip(fleet_reconciliation.as_ref())
            .map(|(revision, reconciliation)| operation_view(revision, reconciliation));

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
            job_selected: status.selected_job.is_some()
                || revision
                    .as_ref()
                    .is_some_and(|revision| revision.selected_job.is_some()),
            operation_available: revision.is_some(),
            manifest_available: manifest.is_some_and(|manifest| !manifest.legs.is_empty()),
            manifest_needs_attention: manifest.is_some_and(FlightManifest::needs_attention),
            fleet_state: fleet_reconciliation
                .as_ref()
                .map_or(FlightOperationStageState::Unavailable, fleet_stage_state),
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
    fleet_state: FlightOperationStageState,
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
        fleet_state: FlightOperationStageState::Unavailable,
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
    let fleet = if evidence.operation_available {
        evidence.fleet_state
    } else {
        FlightOperationStageState::NotStarted
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

fn operation_view(
    revision: &FlightOperationRevision,
    fleet_reconciliation: &FlightOperationFleetReconciliationView,
) -> FlightOperationView {
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
        fleet_reconciliation: fleet_reconciliation.clone(),
    }
}

fn build_fleet_reconciliation(
    revision: &FlightOperationRevision,
    fleet: Option<&FleetSnapshotView>,
) -> FlightOperationFleetReconciliationView {
    let comparison = crate::compare_plan_to_fleet_aircraft(&revision.plan, fleet);
    let manifest_coverage = FlightOperationManifestCoverageView {
        leg_count: revision.manifest.legs.len(),
        passenger_legs_reported: revision
            .manifest
            .legs
            .iter()
            .filter(|leg| leg.passengers.is_some())
            .count(),
        freight_legs_reported: revision
            .manifest
            .legs
            .iter()
            .filter(|leg| leg.freight.is_some())
            .count(),
        source_gaps_present: revision.manifest.needs_attention(),
    };
    let mut findings = comparison
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.category,
                DispatchFindingCategory::AircraftIdentity
                    | DispatchFindingCategory::AircraftModel
                    | DispatchFindingCategory::AircraftPosition
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    findings.push(manifest_coverage_finding(&manifest_coverage));
    findings.extend(capability_gap_findings(&manifest_coverage));

    FlightOperationFleetReconciliationView {
        schema_version: FLEET_RECONCILIATION_SCHEMA_VERSION,
        fleet_available: comparison.fleet_available,
        fleet_observed_at: comparison.fleet_observed_at,
        candidate: comparison.matched_aircraft,
        manifest_coverage,
        findings,
        provenance: comparison.provenance,
    }
}

fn fleet_stage_state(
    reconciliation: &FlightOperationFleetReconciliationView,
) -> FlightOperationStageState {
    if !reconciliation.fleet_available {
        return FlightOperationStageState::Unavailable;
    }
    if reconciliation.provenance.freshness == SnapshotFreshness::Stale {
        return FlightOperationStageState::Stale;
    }
    if reconciliation.candidate.is_none()
        || reconciliation.findings.iter().any(|finding| {
            matches!(
                finding.status,
                DispatchFindingStatus::Difference | DispatchFindingStatus::Unavailable
            )
        })
    {
        FlightOperationStageState::NeedsAttention
    } else {
        FlightOperationStageState::Ready
    }
}

fn manifest_coverage_finding(coverage: &FlightOperationManifestCoverageView) -> DispatchFinding {
    let summary = format!(
        "{} legs; passenger facts on {}; freight facts on {}",
        coverage.leg_count, coverage.passenger_legs_reported, coverage.freight_legs_reported
    );
    if coverage.leg_count == 0 {
        return reconciliation_finding(
            DispatchFindingCategory::ManifestCoverage,
            DispatchFindingStatus::Information,
            "fleet-reconciliation-manifest-empty",
            "No job manifest attached",
            "This operation can still compare aircraft identity and position, but it has no retained job load to reconcile.",
            Some(summary),
            None,
        );
    }
    if coverage.source_gaps_present {
        return reconciliation_finding(
            DispatchFindingCategory::ManifestCoverage,
            DispatchFindingStatus::Unavailable,
            "fleet-reconciliation-manifest-gaps",
            "Manifest has source gaps",
            "One or more job legs did not report their expected passenger count or freight weight.",
            Some(summary),
            None,
        );
    }
    reconciliation_finding(
        DispatchFindingCategory::ManifestCoverage,
        DispatchFindingStatus::Match,
        "fleet-reconciliation-manifest-retained",
        "Manifest evidence retained",
        "The accepted operation retains the reported passenger and freight facts without turning them into an aircraft-capacity claim.",
        Some(summary),
        None,
    )
}

fn capability_gap_findings(coverage: &FlightOperationManifestCoverageView) -> [DispatchFinding; 4] {
    let passenger_context = (coverage.passenger_legs_reported > 0).then(|| {
        format!(
            "Passenger facts on {} legs",
            coverage.passenger_legs_reported
        )
    });
    let freight_context = (coverage.freight_legs_reported > 0)
        .then(|| format!("Freight facts on {} legs", coverage.freight_legs_reported));
    [
        reconciliation_finding(
            DispatchFindingCategory::AircraftSeats,
            DispatchFindingStatus::Unavailable,
            "fleet-reconciliation-seats-unavailable",
            "Seat check unavailable",
            "The verified OnAir fleet contract does not currently provide a certified seat count.",
            passenger_context,
            None,
        ),
        reconciliation_finding(
            DispatchFindingCategory::AircraftPayloadCapacity,
            DispatchFindingStatus::Unavailable,
            "fleet-reconciliation-payload-capacity-unavailable",
            "Payload-capacity check unavailable",
            "The verified OnAir fleet contract does not currently provide certified payload or cargo capacity.",
            freight_context,
            None,
        ),
        reconciliation_finding(
            DispatchFindingCategory::AircraftConfiguration,
            DispatchFindingStatus::Unavailable,
            "fleet-reconciliation-configuration-unavailable",
            "Configuration check unavailable",
            "The current fleet evidence does not prove the selected airframe's passenger or cargo configuration.",
            None,
            None,
        ),
        reconciliation_finding(
            DispatchFindingCategory::AircraftAvailability,
            DispatchFindingStatus::Unavailable,
            "fleet-reconciliation-availability-unavailable",
            "Operational availability unavailable",
            "Location is an observed fact; it does not prove maintenance condition, scheduling availability, or readiness for flight.",
            None,
            None,
        ),
    ]
}

fn reconciliation_finding(
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

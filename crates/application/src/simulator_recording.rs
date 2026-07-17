use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{
    Coordinates, FlightPlanSnapshot, Mass, MassUnit, SimulatorTelemetrySnapshot,
};
use wyrmgrid_storage::{
    SimulatorRecordingPreferencesRecord, SimulatorSampleRecord, SimulatorSessionEventRecord,
    SimulatorSessionRecord, Store,
};

use crate::{SimulatorSessionDebrief, SimulatorTelemetryObserver, build_simulator_debrief};

const DEFAULT_RETENTION_DAYS: u32 = 30;
const MIN_RETENTION_DAYS: u32 = 1;
const MAX_RETENTION_DAYS: u32 = 3_650;
const MAX_SESSION_LIST: u32 = 500;
const MAX_GRAPH_SAMPLES: u32 = 600;
const MAX_ANALYSIS_SAMPLES: u32 = 50_000;
const MAX_DEBRIEF_SOURCE_SAMPLES: u64 = 250_000;
const MAX_EXPORT_SAMPLES: u64 = 250_000;
const GAP_AFTER_SECONDS: i64 = 3;
const DEFAULT_LANDING_SETTLE_SECONDS: u32 = 30;
const MIN_LANDING_SETTLE_SECONDS: u32 = 10;
const MAX_LANDING_SETTLE_SECONDS: u32 = 600;
const AUTOMATIC_TAKEOFF_CONFIRMATION_SAMPLES: usize = 2;
pub const SIMBRIEF_CORRELATION_VERSION: u32 = 2;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SimulatorRecordingError {
    #[error("A simulator recording is already active.")]
    AlreadyRecording,
    #[error("No simulator recording is active.")]
    NotRecording,
    #[error("Fresh simulator telemetry is required before recording can begin.")]
    FreshTelemetryRequired,
    #[error("The recording retention setting is outside supported bounds.")]
    InvalidRetention,
    #[error("The requested simulator recording does not exist.")]
    UnknownSession,
    #[error("The requested simulator recording export is too large.")]
    ExportTooLarge,
    #[error("The requested simulator debrief is too large to project safely.")]
    DebriefTooLarge,
    #[error("The imported flight plan is invalid or cannot be associated.")]
    InvalidPlan,
    #[error("Stop the active recording before deleting it.")]
    ActiveSession,
    #[error("WyrmGrid could not read or update its local simulator recordings.")]
    StorageUnavailable,
    #[error("The simulator recording state is unavailable.")]
    StateUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatorRecordingPreferences {
    pub retention_days: u32,
    pub automatic_start: bool,
    pub automatic_stop: bool,
    pub landing_settle_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorCaptureMode {
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorRecordingStatus {
    Active,
    Completed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulatorSessionSummary {
    pub id: String,
    pub provider_id: String,
    pub simulator_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulator_version: Option<String>,
    pub aircraft_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_registration: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub status: SimulatorRecordingStatus,
    pub sample_count: u64,
    pub capture_mode: SimulatorCaptureMode,
    pub pinned: bool,
    pub plan_associated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorRecordedSample {
    pub source_sequence: u64,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_time_utc: Option<String>,
    pub altitude_feet: f64,
    pub indicated_airspeed_knots: f64,
    pub true_airspeed_knots: f64,
    pub ground_speed_knots: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_total_weight_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_weight_pounds: Option<f64>,
    pub pitch_degrees: f64,
    pub bank_degrees: f64,
    pub gap_before: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Coordinates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_ground: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines_running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parking_brake_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorSessionEvent {
    pub id: i64,
    pub event_kind: String,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_sequence: Option<u64>,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightPlanAssociation {
    pub correlation_version: u32,
    pub plan_id: String,
    pub origin_icao: String,
    pub destination_icao: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_plan_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlannedActualComparison {
    pub association: FlightPlanAssociation,
    pub analysis_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_enroute_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_distance_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_track_distance_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_initial_altitude_ft: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_peak_altitude_ft: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_takeoff_fuel_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_landing_fuel_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_start_fuel_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_end_fuel_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_fuel_used_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_delta_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_delta_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude_delta_ft: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub takeoff_fuel_delta_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing_fuel_delta_pounds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_proximity_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_proximity_nm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_matches: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorRecordingView {
    pub preferences: SimulatorRecordingPreferences,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    pub sessions: Vec<SimulatorSessionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulatorSessionView {
    pub session: SimulatorSessionSummary,
    pub samples: Vec<SimulatorRecordedSample>,
    pub sample_window_limit: u32,
    pub sample_window_offset: u32,
    pub has_older_samples: bool,
    pub has_newer_samples: bool,
    pub events: Vec<SimulatorSessionEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<PlannedActualComparison>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulatorRecordingExport {
    pub filename: String,
    pub media_type: String,
    pub content: String,
}

#[derive(Clone)]
pub struct SimulatorRecordingService {
    inner: Arc<SimulatorRecordingInner>,
}

struct SimulatorRecordingInner {
    store: Store,
    active: Mutex<Option<ActiveRecording>>,
    preferences: RwLock<SimulatorRecordingPreferences>,
    lifecycle: Mutex<LifecycleEvidence>,
    active_plan: RwLock<Option<FlightPlanSnapshot>>,
    last_code: Mutex<Option<String>>,
}

struct ActiveRecording {
    id: String,
    provider_id: String,
    aircraft_title: String,
    aircraft_registration: Option<String>,
    last_sequence: u64,
    last_observed_at: DateTime<Utc>,
    capture_mode: SimulatorCaptureMode,
    grounded_since: Option<DateTime<Utc>>,
    grounded_samples: u32,
}

#[derive(Default)]
struct LifecycleEvidence {
    provider_id: Option<String>,
    aircraft_title: Option<String>,
    aircraft_registration: Option<String>,
    airborne_candidates: Vec<SimulatorTelemetrySnapshot>,
}

impl SimulatorRecordingService {
    pub fn new(store: Store) -> Self {
        let stored_preferences = store
            .load_simulator_recording_preferences_record()
            .ok()
            .flatten()
            .map(preferences_from_record)
            .unwrap_or_else(default_preferences);
        let service = Self {
            inner: Arc::new(SimulatorRecordingInner {
                store,
                active: Mutex::new(None),
                preferences: RwLock::new(stored_preferences),
                lifecycle: Mutex::new(LifecycleEvidence::default()),
                active_plan: RwLock::new(None),
                last_code: Mutex::new(None),
            }),
        };
        let now = timestamp(Utc::now());
        let recovery_result = service
            .inner
            .store
            .interrupt_active_simulator_sessions(&now)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)
            .and_then(|_| service.prune_expired());
        if recovery_result.is_err() {
            service.set_last_code("recording.storage_failed");
        }
        service
    }

    pub fn status(&self) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        let preferences = self.preferences()?;
        let active_session_id = self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .as_ref()
            .map(|active| active.id.clone());
        let sessions = self
            .inner
            .store
            .list_simulator_session_records(MAX_SESSION_LIST)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(session_from_record)
            .collect::<Result<Vec<_>, _>>()?;
        let last_code = self
            .inner
            .last_code
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .clone();
        Ok(SimulatorRecordingView {
            preferences,
            active_session_id,
            sessions,
            last_code,
        })
    }

    pub fn preferences(&self) -> Result<SimulatorRecordingPreferences, SimulatorRecordingError> {
        self.inner
            .preferences
            .read()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)
            .map(|preferences| preferences.clone())
    }

    pub fn update_preferences(
        &self,
        preferences: SimulatorRecordingPreferences,
    ) -> Result<SimulatorRecordingPreferences, SimulatorRecordingError> {
        if !(MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(&preferences.retention_days) {
            return Err(SimulatorRecordingError::InvalidRetention);
        }
        if !(MIN_LANDING_SETTLE_SECONDS..=MAX_LANDING_SETTLE_SECONDS)
            .contains(&preferences.landing_settle_seconds)
        {
            return Err(SimulatorRecordingError::InvalidRetention);
        }
        self.inner
            .store
            .save_simulator_recording_preferences_record(&SimulatorRecordingPreferencesRecord {
                retention_days: preferences.retention_days,
                automatic_start: preferences.automatic_start,
                automatic_stop: preferences.automatic_stop,
                landing_settle_seconds: preferences.landing_settle_seconds,
            })
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        *self
            .inner
            .preferences
            .write()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)? = preferences.clone();
        if !preferences.automatic_start {
            self.reset_lifecycle();
        }
        self.prune_expired()?;
        Ok(preferences)
    }

    pub fn start(
        &self,
        provider_id: &str,
        snapshot: Option<SimulatorTelemetrySnapshot>,
    ) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        let snapshot = snapshot.ok_or(SimulatorRecordingError::FreshTelemetryRequired)?;
        snapshot
            .validate()
            .map_err(|_| SimulatorRecordingError::FreshTelemetryRequired)?;
        let mut active = self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?;
        if active.is_some() {
            return Err(SimulatorRecordingError::AlreadyRecording);
        }

        self.create_recording(
            &mut active,
            provider_id,
            vec![snapshot],
            SimulatorCaptureMode::Manual,
        )?;
        drop(active);
        self.clear_last_code()?;
        self.status()
    }

    pub fn stop(&self) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        let mut active = self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?;
        let recording = active
            .as_ref()
            .ok_or(SimulatorRecordingError::NotRecording)?;
        self.inner
            .store
            .finish_simulator_session_record(&recording.id, &timestamp(Utc::now()), "completed")
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        self.append_event(
            &recording.id,
            "recording_stopped_manually",
            Utc::now(),
            None,
            serde_json::json!({ "source": "user" }),
        )?;
        *active = None;
        drop(active);
        self.clear_last_code()?;
        self.status()
    }

    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<SimulatorSessionView, SimulatorRecordingError> {
        self.session_window(session_id, 0)
    }

    pub fn debrief(
        &self,
        session_id: &str,
    ) -> Result<SimulatorSessionDebrief, SimulatorRecordingError> {
        let record = self.session_record(session_id)?;
        if record.sample_count > MAX_DEBRIEF_SOURCE_SAMPLES {
            return Err(SimulatorRecordingError::DebriefTooLarge);
        }
        let plan = record
            .plan_snapshot_json
            .as_deref()
            .map(serde_json::from_str::<FlightPlanSnapshot>)
            .transpose()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        let session = session_from_record(record)?;
        let sample_limit = u32::try_from(session.sample_count)
            .map_err(|_| SimulatorRecordingError::DebriefTooLarge)?;
        let samples = self
            .inner
            .store
            .list_simulator_sample_record_window(session_id, 0, sample_limit)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(sample_from_record)
            .collect::<Vec<_>>();
        let comparison = plan
            .as_ref()
            .map(|plan| comparison_from_samples(&session, plan, &samples, true))
            .transpose()?;
        Ok(build_simulator_debrief(
            session,
            &samples,
            plan.as_ref(),
            comparison,
        ))
    }

    pub fn session_window(
        &self,
        session_id: &str,
        sample_offset: u32,
    ) -> Result<SimulatorSessionView, SimulatorRecordingError> {
        let record = self.session_record(session_id)?;
        let plan = record
            .plan_snapshot_json
            .as_deref()
            .map(serde_json::from_str::<FlightPlanSnapshot>)
            .transpose()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        let session = session_from_record(record)?;
        let samples = self
            .inner
            .store
            .list_simulator_sample_record_window(session_id, sample_offset, MAX_GRAPH_SAMPLES)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(sample_from_record)
            .collect::<Vec<_>>();
        let events = self
            .inner
            .store
            .list_simulator_session_event_records(session_id)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(event_from_record)
            .collect::<Result<Vec<_>, _>>()?;
        let comparison = plan
            .as_ref()
            .map(|plan| self.compare_plan(session_id, &session, plan))
            .transpose()?;
        Ok(SimulatorSessionView {
            has_older_samples: u64::from(sample_offset) + u64::from(MAX_GRAPH_SAMPLES)
                < session.sample_count,
            has_newer_samples: sample_offset > 0,
            session,
            samples,
            sample_window_limit: MAX_GRAPH_SAMPLES,
            sample_window_offset: sample_offset,
            events,
            comparison,
        })
    }

    pub fn set_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        if !self
            .inner
            .store
            .set_simulator_session_pinned(session_id, pinned)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
        {
            return Err(SimulatorRecordingError::UnknownSession);
        }
        self.status()
    }

    pub fn set_plan_context(
        &self,
        plan: Option<FlightPlanSnapshot>,
    ) -> Result<(), SimulatorRecordingError> {
        if plan.as_ref().is_some_and(|plan| plan.validate().is_err()) {
            return Err(SimulatorRecordingError::InvalidPlan);
        }
        *self
            .inner
            .active_plan
            .write()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)? = plan.clone();
        let Some(plan) = plan else {
            return Ok(());
        };
        let active_session_id = self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .as_ref()
            .map(|recording| recording.id.clone());
        if let Some(session_id) = active_session_id {
            self.attach_plan(&session_id, &plan)?;
            self.append_event(
                &session_id,
                "flight_plan_associated",
                Utc::now(),
                None,
                plan_evidence(&plan),
            )?;
        }
        Ok(())
    }

    pub fn export_session(
        &self,
        session_id: &str,
        format: SimulatorExportFormat,
    ) -> Result<SimulatorRecordingExport, SimulatorRecordingError> {
        let session = self
            .inner
            .store
            .list_simulator_session_records(MAX_SESSION_LIST)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or(SimulatorRecordingError::UnknownSession)?;
        if session.sample_count > MAX_EXPORT_SAMPLES {
            return Err(SimulatorRecordingError::ExportTooLarge);
        }
        let plan_snapshot = session
            .plan_snapshot_json
            .as_deref()
            .map(serde_json::from_str::<serde_json::Value>)
            .transpose()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        let samples = self
            .inner
            .store
            .list_simulator_sample_record_window(session_id, 0, session.sample_count as u32)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(sample_from_record)
            .collect::<Vec<_>>();
        let stem = format!("wyrmgrid-flight-{}", session.id);
        match format {
            SimulatorExportFormat::Json => {
                let view = self.session_window(session_id, 0)?;
                let content = serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version": 1,
                    "session": view.session,
                    "samples": samples,
                    "events": view.events,
                    "comparison": view.comparison,
                    "plan_snapshot": plan_snapshot,
                }))
                .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
                Ok(SimulatorRecordingExport {
                    filename: format!("{stem}.json"),
                    media_type: "application/json".into(),
                    content,
                })
            }
            SimulatorExportFormat::Csv => Ok(SimulatorRecordingExport {
                filename: format!("{stem}.csv"),
                media_type: "text/csv".into(),
                content: samples_to_csv(&samples),
            }),
        }
    }

    pub fn delete_session(
        &self,
        session_id: &str,
    ) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        if self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .as_ref()
            .is_some_and(|active| active.id == session_id)
        {
            return Err(SimulatorRecordingError::ActiveSession);
        }
        if !self
            .status()?
            .sessions
            .iter()
            .any(|session| session.id == session_id)
        {
            return Err(SimulatorRecordingError::UnknownSession);
        }
        self.inner
            .store
            .delete_simulator_session_record(session_id)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        self.status()
    }

    pub fn delete_all(&self) -> Result<SimulatorRecordingView, SimulatorRecordingError> {
        if self
            .inner
            .active
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .is_some()
        {
            return Err(SimulatorRecordingError::ActiveSession);
        }
        self.inner
            .store
            .delete_all_simulator_session_records()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        self.status()
    }

    fn observe_snapshot(&self, provider_id: &str, snapshot: &SimulatorTelemetrySnapshot) {
        let Ok(mut active) = self.inner.active.lock() else {
            return;
        };
        let Some(recording) = active.as_mut() else {
            drop(active);
            self.observe_lifecycle_candidate(provider_id, snapshot);
            return;
        };
        if recording.provider_id != provider_id
            || recording.aircraft_title != snapshot.aircraft.title
            || recording.aircraft_registration != snapshot.aircraft.registration
        {
            let _ = self.inner.store.finish_simulator_session_record(
                &recording.id,
                &timestamp(snapshot.provenance.retrieved_at),
                "interrupted",
            );
            *active = None;
            self.set_last_code("recording.aircraft_changed");
            return;
        }
        if snapshot.sequence == recording.last_sequence {
            return;
        }
        let elapsed = snapshot
            .provenance
            .retrieved_at
            .signed_duration_since(recording.last_observed_at)
            .num_seconds();
        let gap_before = snapshot.sequence != recording.last_sequence.saturating_add(1)
            || !(0..=GAP_AFTER_SECONDS).contains(&elapsed);
        if gap_before {
            let _ = self.append_event(
                &recording.id,
                "telemetry_gap",
                snapshot.provenance.retrieved_at,
                Some(snapshot.sequence),
                serde_json::json!({
                    "previous_sequence": recording.last_sequence,
                    "previous_observed_at": timestamp(recording.last_observed_at),
                }),
            );
        }
        match self.inner.store.append_simulator_sample_record(
            &recording.id,
            &sample_from_snapshot(snapshot, gap_before),
        ) {
            Ok(_) => {
                recording.last_sequence = snapshot.sequence;
                recording.last_observed_at = snapshot.provenance.retrieved_at;
                if gap_before {
                    recording.grounded_since = None;
                    recording.grounded_samples = 0;
                }
                if recording.capture_mode == SimulatorCaptureMode::Automatic
                    && let Some(id) = self.automatic_landing_settled(recording, snapshot)
                {
                    match self.inner.store.finish_simulator_session_record(
                        &id,
                        &timestamp(snapshot.provenance.retrieved_at),
                        "completed",
                    ) {
                        Ok(()) => {
                            *active = None;
                            self.set_last_code("recording.automatic_completed");
                        }
                        Err(_) => {
                            *active = None;
                            self.set_last_code("recording.storage_failed");
                        }
                    }
                }
            }
            Err(_) => {
                let _ = self.inner.store.finish_simulator_session_record(
                    &recording.id,
                    &timestamp(Utc::now()),
                    "interrupted",
                );
                *active = None;
                self.set_last_code("recording.storage_failed");
            }
        }
    }

    fn create_recording(
        &self,
        active: &mut Option<ActiveRecording>,
        provider_id: &str,
        snapshots: Vec<SimulatorTelemetrySnapshot>,
        capture_mode: SimulatorCaptureMode,
    ) -> Result<String, SimulatorRecordingError> {
        let first = snapshots
            .first()
            .ok_or(SimulatorRecordingError::FreshTelemetryRequired)?;
        let last = snapshots
            .last()
            .ok_or(SimulatorRecordingError::FreshTelemetryRequired)?;
        let plan = self
            .inner
            .active_plan
            .read()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)?
            .clone();
        let plan_snapshot_json = plan
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| SimulatorRecordingError::InvalidPlan)?;
        let id = Uuid::new_v4().to_string();
        self.inner
            .store
            .create_simulator_session_record(&SimulatorSessionRecord {
                id: id.clone(),
                provider_id: provider_id.to_owned(),
                simulator_family: first.simulator.family.clone(),
                simulator_version: first.simulator.version.clone(),
                aircraft_title: first.aircraft.title.clone(),
                aircraft_registration: first.aircraft.registration.clone(),
                started_at: timestamp(first.provenance.retrieved_at),
                ended_at: None,
                origin: match capture_mode {
                    SimulatorCaptureMode::Manual => "manual",
                    SimulatorCaptureMode::Automatic => "automatic",
                }
                .into(),
                status: "active".into(),
                sample_count: 0,
                pinned: false,
                plan_snapshot_json,
            })
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        for (index, snapshot) in snapshots.iter().enumerate() {
            if self
                .inner
                .store
                .append_simulator_sample_record(
                    &id,
                    &sample_from_snapshot(
                        snapshot,
                        index > 0
                            && snapshot.sequence != snapshots[index - 1].sequence.saturating_add(1),
                    ),
                )
                .is_err()
            {
                let _ = self.inner.store.finish_simulator_session_record(
                    &id,
                    &timestamp(Utc::now()),
                    "interrupted",
                );
                return Err(SimulatorRecordingError::StorageUnavailable);
            }
        }
        *active = Some(ActiveRecording {
            id: id.clone(),
            provider_id: provider_id.to_owned(),
            aircraft_title: last.aircraft.title.clone(),
            aircraft_registration: last.aircraft.registration.clone(),
            last_sequence: last.sequence,
            last_observed_at: last.provenance.retrieved_at,
            capture_mode,
            grounded_since: None,
            grounded_samples: 0,
        });
        let event_kind = match capture_mode {
            SimulatorCaptureMode::Manual => "recording_started_manually",
            SimulatorCaptureMode::Automatic => "takeoff_confirmed",
        };
        self.append_event(
            &id,
            event_kind,
            last.provenance.retrieved_at,
            Some(last.sequence),
            serde_json::json!({
                "on_ground": last.on_ground,
                "confirmation_samples": snapshots.len(),
                "source": "simulator_telemetry",
            }),
        )?;
        if let Some(plan) = plan {
            self.append_event(
                &id,
                "flight_plan_associated",
                last.provenance.retrieved_at,
                Some(last.sequence),
                plan_evidence(&plan),
            )?;
        }
        Ok(id)
    }

    fn observe_lifecycle_candidate(
        &self,
        provider_id: &str,
        snapshot: &SimulatorTelemetrySnapshot,
    ) {
        let Ok(preferences) = self.preferences() else {
            return;
        };
        if !preferences.automatic_start
            || snapshot.on_ground
            || snapshot.paused == Some(true)
            || snapshot.simulation_rate == Some(0.0)
        {
            self.reset_lifecycle();
            return;
        }
        let Ok(mut lifecycle) = self.inner.lifecycle.lock() else {
            return;
        };
        let same_candidate = lifecycle.provider_id.as_deref() == Some(provider_id)
            && lifecycle.aircraft_title.as_deref() == Some(&snapshot.aircraft.title)
            && lifecycle.aircraft_registration == snapshot.aircraft.registration;
        if !same_candidate {
            *lifecycle = LifecycleEvidence {
                provider_id: Some(provider_id.to_owned()),
                aircraft_title: Some(snapshot.aircraft.title.clone()),
                aircraft_registration: snapshot.aircraft.registration.clone(),
                airborne_candidates: Vec::new(),
            };
        }
        if let Some(previous) = lifecycle.airborne_candidates.last() {
            if snapshot.sequence <= previous.sequence {
                return;
            }
            let elapsed = snapshot
                .provenance
                .retrieved_at
                .signed_duration_since(previous.provenance.retrieved_at)
                .num_seconds();
            if snapshot.sequence != previous.sequence.saturating_add(1)
                || !(0..=GAP_AFTER_SECONDS).contains(&elapsed)
            {
                lifecycle.airborne_candidates.clear();
            }
        }
        lifecycle.airborne_candidates.push(snapshot.clone());
        if lifecycle.airborne_candidates.len() < AUTOMATIC_TAKEOFF_CONFIRMATION_SAMPLES {
            return;
        }
        let candidates = std::mem::take(&mut lifecycle.airborne_candidates);
        drop(lifecycle);
        let Ok(mut active) = self.inner.active.lock() else {
            return;
        };
        if active.is_none()
            && self
                .create_recording(
                    &mut active,
                    provider_id,
                    candidates,
                    SimulatorCaptureMode::Automatic,
                )
                .is_ok()
        {
            let _ = self.clear_last_code();
        }
        self.reset_lifecycle();
    }

    fn automatic_landing_settled(
        &self,
        recording: &mut ActiveRecording,
        snapshot: &SimulatorTelemetrySnapshot,
    ) -> Option<String> {
        let Ok(preferences) = self.preferences() else {
            return None;
        };
        if !preferences.automatic_stop {
            recording.grounded_since = None;
            recording.grounded_samples = 0;
            return None;
        }
        if !snapshot.on_ground
            || snapshot.paused == Some(true)
            || snapshot.simulation_rate == Some(0.0)
        {
            recording.grounded_since = None;
            recording.grounded_samples = 0;
            return None;
        }
        let grounded_since = *recording
            .grounded_since
            .get_or_insert(snapshot.provenance.retrieved_at);
        recording.grounded_samples = recording.grounded_samples.saturating_add(1);
        let settled = recording.grounded_samples >= 2
            && snapshot
                .provenance
                .retrieved_at
                .signed_duration_since(grounded_since)
                .num_seconds()
                >= i64::from(preferences.landing_settle_seconds);
        if !settled {
            return None;
        }
        let id = recording.id.clone();
        let _ = self.append_event(
            &id,
            "landing_settled",
            snapshot.provenance.retrieved_at,
            Some(snapshot.sequence),
            serde_json::json!({
                "on_ground": true,
                "settle_seconds": preferences.landing_settle_seconds,
                "confirmation_samples": recording.grounded_samples,
            }),
        );
        Some(id)
    }

    fn reset_lifecycle(&self) {
        if let Ok(mut lifecycle) = self.inner.lifecycle.lock() {
            *lifecycle = LifecycleEvidence::default();
        }
    }

    fn attach_plan(
        &self,
        session_id: &str,
        plan: &FlightPlanSnapshot,
    ) -> Result<(), SimulatorRecordingError> {
        let json = serde_json::to_string(plan).map_err(|_| SimulatorRecordingError::InvalidPlan)?;
        if !self
            .inner
            .store
            .attach_simulator_session_plan(session_id, &json, SIMBRIEF_CORRELATION_VERSION)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
        {
            return Err(SimulatorRecordingError::UnknownSession);
        }
        Ok(())
    }

    fn append_event(
        &self,
        session_id: &str,
        event_kind: &str,
        observed_at: DateTime<Utc>,
        source_sequence: Option<u64>,
        evidence: serde_json::Value,
    ) -> Result<(), SimulatorRecordingError> {
        self.inner
            .store
            .append_simulator_session_event_record(
                session_id,
                &SimulatorSessionEventRecord {
                    id: 0,
                    event_kind: event_kind.to_owned(),
                    observed_at: timestamp(observed_at),
                    source_sequence,
                    evidence_json: serde_json::to_string(&evidence)
                        .map_err(|_| SimulatorRecordingError::StorageUnavailable)?,
                },
            )
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)
    }

    fn compare_plan(
        &self,
        session_id: &str,
        session: &SimulatorSessionSummary,
        plan: &FlightPlanSnapshot,
    ) -> Result<PlannedActualComparison, SimulatorRecordingError> {
        plan.validate()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        let analysis_complete = session.sample_count <= u64::from(MAX_ANALYSIS_SAMPLES);
        let samples = if analysis_complete {
            self.inner
                .store
                .list_simulator_sample_record_window(
                    session_id,
                    0,
                    u32::try_from(session.sample_count).unwrap_or(MAX_ANALYSIS_SAMPLES),
                )
                .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
                .into_iter()
                .map(sample_from_record)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        comparison_from_samples(session, plan, &samples, analysis_complete)
    }

    fn session_record(
        &self,
        session_id: &str,
    ) -> Result<SimulatorSessionRecord, SimulatorRecordingError> {
        self.inner
            .store
            .list_simulator_session_records(MAX_SESSION_LIST)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or(SimulatorRecordingError::UnknownSession)
    }

    fn prune_expired(&self) -> Result<(), SimulatorRecordingError> {
        let retention_days = self.preferences()?.retention_days;
        let before = Utc::now() - Duration::days(i64::from(retention_days));
        self.inner
            .store
            .prune_simulator_session_records(&timestamp(before))
            .map(|_| ())
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)
    }

    fn clear_last_code(&self) -> Result<(), SimulatorRecordingError> {
        *self
            .inner
            .last_code
            .lock()
            .map_err(|_| SimulatorRecordingError::StateUnavailable)? = None;
        Ok(())
    }

    fn set_last_code(&self, code: &str) {
        if let Ok(mut last_code) = self.inner.last_code.lock() {
            *last_code = Some(code.to_owned());
        }
    }
}

fn comparison_from_samples(
    session: &SimulatorSessionSummary,
    plan: &FlightPlanSnapshot,
    samples: &[SimulatorRecordedSample],
    analysis_complete: bool,
) -> Result<PlannedActualComparison, SimulatorRecordingError> {
    plan.validate()
        .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
    let airports = &plan.airports.value;
    let route = plan.route.as_ref().map(|group| &group.value);
    let recorded_seconds = samples
        .first()
        .zip(samples.last())
        .and_then(|(first, last)| {
            DateTime::parse_from_rfc3339(&last.observed_at)
                .ok()?
                .signed_duration_since(DateTime::parse_from_rfc3339(&first.observed_at).ok()?)
                .num_seconds()
                .try_into()
                .ok()
        });
    let recorded_track_distance_nm = analysis_complete
        .then(|| track_distance_nm(samples))
        .flatten();
    let recorded_peak_altitude_ft = analysis_complete.then(|| {
        samples
            .iter()
            .map(|sample| sample.altitude_feet)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let recorded_peak_altitude_ft =
        recorded_peak_altitude_ft.filter(|altitude| altitude.is_finite());
    let recorded_fuel_used_pounds = analysis_complete
        .then(|| fuel_used_pounds(samples))
        .flatten();
    let recorded_start_fuel_pounds = samples
        .iter()
        .find_map(|sample| sample.fuel_total_weight_pounds);
    let recorded_end_fuel_pounds = samples
        .iter()
        .rev()
        .find_map(|sample| sample.fuel_total_weight_pounds);
    let planned_takeoff_fuel_pounds = plan
        .fuel
        .as_ref()
        .and_then(|group| group.value.takeoff)
        .map(mass_in_pounds);
    let planned_landing_fuel_pounds = plan
        .fuel
        .as_ref()
        .and_then(|group| group.value.landing)
        .map(mass_in_pounds);
    let planned_enroute_seconds = plan
        .schedule
        .as_ref()
        .and_then(|group| group.value.estimated_enroute_seconds);
    let planned_distance_nm = route.and_then(|route| route.distance_nm);
    let planned_initial_altitude_ft = route.and_then(|route| route.initial_altitude_ft);
    let first_position = samples.iter().find_map(|sample| sample.position);
    let last_position = samples.iter().rev().find_map(|sample| sample.position);

    Ok(PlannedActualComparison {
        association: FlightPlanAssociation {
            correlation_version: SIMBRIEF_CORRELATION_VERSION,
            plan_id: plan.id.0.to_string(),
            origin_icao: airports.origin.icao.clone(),
            destination_icao: airports.destination.icao.clone(),
            provider_plan_reference: plan.identity.value.provider_plan_reference.clone(),
        },
        analysis_complete,
        planned_enroute_seconds,
        recorded_seconds,
        planned_distance_nm,
        recorded_track_distance_nm,
        planned_initial_altitude_ft,
        recorded_peak_altitude_ft,
        planned_takeoff_fuel_pounds,
        planned_landing_fuel_pounds,
        recorded_start_fuel_pounds,
        recorded_end_fuel_pounds,
        recorded_fuel_used_pounds,
        duration_delta_seconds: recorded_seconds
            .zip(planned_enroute_seconds)
            .map(|(recorded, planned)| recorded as i64 - i64::from(planned)),
        distance_delta_nm: recorded_track_distance_nm
            .zip(planned_distance_nm)
            .map(|(recorded, planned)| recorded - planned),
        altitude_delta_ft: recorded_peak_altitude_ft
            .zip(planned_initial_altitude_ft)
            .map(|(recorded, planned)| recorded - f64::from(planned)),
        takeoff_fuel_delta_pounds: recorded_start_fuel_pounds
            .zip(planned_takeoff_fuel_pounds)
            .map(|(recorded, planned)| recorded - planned),
        landing_fuel_delta_pounds: recorded_end_fuel_pounds
            .zip(planned_landing_fuel_pounds)
            .map(|(recorded, planned)| recorded - planned),
        origin_proximity_nm: airports
            .origin
            .location
            .zip(first_position)
            .map(|(airport, observed)| distance_nm(airport, observed)),
        destination_proximity_nm: airports
            .destination
            .location
            .zip(last_position)
            .map(|(airport, observed)| distance_nm(airport, observed)),
        registration_matches: plan
            .aircraft
            .as_ref()
            .and_then(|group| group.value.registration.as_deref())
            .zip(session.aircraft_registration.as_deref())
            .map(|(planned, observed)| planned.eq_ignore_ascii_case(observed)),
    })
}

impl SimulatorTelemetryObserver for SimulatorRecordingService {
    fn observe(&self, provider_id: &str, snapshot: &SimulatorTelemetrySnapshot) {
        self.observe_snapshot(provider_id, snapshot);
    }
}

fn session_from_record(
    record: SimulatorSessionRecord,
) -> Result<SimulatorSessionSummary, SimulatorRecordingError> {
    let status = match record.status.as_str() {
        "active" => SimulatorRecordingStatus::Active,
        "completed" => SimulatorRecordingStatus::Completed,
        "interrupted" => SimulatorRecordingStatus::Interrupted,
        _ => return Err(SimulatorRecordingError::StorageUnavailable),
    };
    let capture_mode = match record.origin.as_str() {
        "manual" => SimulatorCaptureMode::Manual,
        "automatic" => SimulatorCaptureMode::Automatic,
        _ => return Err(SimulatorRecordingError::StorageUnavailable),
    };
    Ok(SimulatorSessionSummary {
        id: record.id,
        provider_id: record.provider_id,
        simulator_family: record.simulator_family,
        simulator_version: record.simulator_version,
        aircraft_title: record.aircraft_title,
        aircraft_registration: record.aircraft_registration,
        started_at: record.started_at,
        ended_at: record.ended_at,
        status,
        sample_count: record.sample_count,
        capture_mode,
        pinned: record.pinned,
        plan_associated: record.plan_snapshot_json.is_some(),
    })
}

fn sample_from_snapshot(
    snapshot: &SimulatorTelemetrySnapshot,
    gap_before: bool,
) -> SimulatorSampleRecord {
    SimulatorSampleRecord {
        source_sequence: snapshot.sequence,
        observed_at: timestamp(snapshot.provenance.retrieved_at),
        simulation_time_utc: snapshot.simulation_time_utc.map(timestamp),
        altitude_feet: snapshot.altitude_feet,
        indicated_airspeed_knots: snapshot.indicated_airspeed_knots,
        true_airspeed_knots: snapshot.true_airspeed_knots,
        ground_speed_knots: snapshot.ground_speed_knots,
        fuel_total_weight_pounds: snapshot.fuel_total_weight_pounds,
        gross_weight_pounds: snapshot.gross_weight_pounds,
        pitch_degrees: snapshot.pitch_degrees,
        bank_degrees: snapshot.bank_degrees,
        gap_before,
        latitude: Some(snapshot.position.latitude),
        longitude: Some(snapshot.position.longitude),
        on_ground: Some(snapshot.on_ground),
        engines_running: snapshot.engines_running,
        parking_brake_set: snapshot.parking_brake_set,
        paused: snapshot.paused,
    }
}

fn sample_from_record(record: SimulatorSampleRecord) -> SimulatorRecordedSample {
    SimulatorRecordedSample {
        source_sequence: record.source_sequence,
        observed_at: record.observed_at,
        simulation_time_utc: record.simulation_time_utc,
        altitude_feet: record.altitude_feet,
        indicated_airspeed_knots: record.indicated_airspeed_knots,
        true_airspeed_knots: record.true_airspeed_knots,
        ground_speed_knots: record.ground_speed_knots,
        fuel_total_weight_pounds: record.fuel_total_weight_pounds,
        gross_weight_pounds: record.gross_weight_pounds,
        pitch_degrees: record.pitch_degrees,
        bank_degrees: record.bank_degrees,
        gap_before: record.gap_before,
        position: record
            .latitude
            .zip(record.longitude)
            .map(|(latitude, longitude)| Coordinates {
                latitude,
                longitude,
            }),
        on_ground: record.on_ground,
        engines_running: record.engines_running,
        parking_brake_set: record.parking_brake_set,
        paused: record.paused,
    }
}

fn event_from_record(
    record: SimulatorSessionEventRecord,
) -> Result<SimulatorSessionEvent, SimulatorRecordingError> {
    Ok(SimulatorSessionEvent {
        id: record.id,
        event_kind: record.event_kind,
        observed_at: record.observed_at,
        source_sequence: record.source_sequence,
        evidence: serde_json::from_str(&record.evidence_json)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?,
    })
}

fn default_preferences() -> SimulatorRecordingPreferences {
    SimulatorRecordingPreferences {
        retention_days: DEFAULT_RETENTION_DAYS,
        automatic_start: false,
        automatic_stop: true,
        landing_settle_seconds: DEFAULT_LANDING_SETTLE_SECONDS,
    }
}

fn preferences_from_record(
    record: SimulatorRecordingPreferencesRecord,
) -> SimulatorRecordingPreferences {
    SimulatorRecordingPreferences {
        retention_days: record.retention_days,
        automatic_start: record.automatic_start,
        automatic_stop: record.automatic_stop,
        landing_settle_seconds: record.landing_settle_seconds,
    }
}

fn plan_evidence(plan: &FlightPlanSnapshot) -> serde_json::Value {
    serde_json::json!({
        "correlation_version": SIMBRIEF_CORRELATION_VERSION,
        "plan_id": plan.id.0.to_string(),
        "origin_icao": plan.airports.value.origin.icao,
        "destination_icao": plan.airports.value.destination.icao,
    })
}

fn mass_in_pounds(mass: Mass) -> f64 {
    match mass.unit {
        MassUnit::Pounds => mass.value,
        MassUnit::Kilograms => mass.value * 2.204_622_621_8,
    }
}

fn track_distance_nm(samples: &[SimulatorRecordedSample]) -> Option<f64> {
    let mut total = 0.0;
    let mut previous = None;
    let mut segments = 0_u64;
    for sample in samples {
        let Some(position) = sample.position else {
            previous = None;
            continue;
        };
        if sample.gap_before {
            previous = None;
        }
        if let Some(from) = previous {
            total += distance_nm(from, position);
            segments = segments.saturating_add(1);
        }
        previous = Some(position);
    }
    (segments > 0).then_some(total)
}

fn fuel_used_pounds(samples: &[SimulatorRecordedSample]) -> Option<f64> {
    const FUEL_INCREASE_TOLERANCE_POUNDS: f64 = 0.5;
    let fuel = samples
        .iter()
        .filter_map(|sample| sample.fuel_total_weight_pounds)
        .collect::<Vec<_>>();
    let first = *fuel.first()?;
    let last = *fuel.last()?;
    if fuel
        .windows(2)
        .any(|pair| pair[1] > pair[0] + FUEL_INCREASE_TOLERANCE_POUNDS)
    {
        return None;
    }
    (first >= last).then_some(first - last)
}

fn distance_nm(from: Coordinates, to: Coordinates) -> f64 {
    const EARTH_RADIUS_NM: f64 = 3_440.065;
    let latitude_delta = (to.latitude - from.latitude).to_radians();
    let longitude_delta = (to.longitude - from.longitude).to_radians();
    let from_latitude = from.latitude.to_radians();
    let to_latitude = to.latitude.to_radians();
    let a = (latitude_delta / 2.0).sin().powi(2)
        + from_latitude.cos() * to_latitude.cos() * (longitude_delta / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_NM * a.sqrt().asin()
}

fn samples_to_csv(samples: &[SimulatorRecordedSample]) -> String {
    let mut csv = String::from(
        "source_sequence,observed_at,simulation_time_utc,latitude,longitude,altitude_feet,indicated_airspeed_knots,true_airspeed_knots,ground_speed_knots,fuel_total_weight_pounds,gross_weight_pounds,pitch_degrees,bank_degrees,on_ground,engines_running,parking_brake_set,paused,gap_before\n",
    );
    for sample in samples {
        let position = sample.position;
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            sample.source_sequence,
            csv_field(&sample.observed_at),
            csv_field(sample.simulation_time_utc.as_deref().unwrap_or("")),
            optional_number(position.map(|value| value.latitude)),
            optional_number(position.map(|value| value.longitude)),
            sample.altitude_feet,
            sample.indicated_airspeed_knots,
            sample.true_airspeed_knots,
            sample.ground_speed_knots,
            optional_number(sample.fuel_total_weight_pounds),
            optional_number(sample.gross_weight_pounds),
            sample.pitch_degrees,
            sample.bank_degrees,
            optional_bool(sample.on_ground),
            optional_bool(sample.engines_running),
            optional_bool(sample.parking_brake_set),
            optional_bool(sample.paused),
            sample.gap_before,
        ));
    }
    csv
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn optional_number(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn optional_bool(value: Option<bool>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
#[path = "tests/simulator_recording.rs"]
mod tests;

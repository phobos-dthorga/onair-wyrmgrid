use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::SimulatorTelemetrySnapshot;
use wyrmgrid_storage::{
    SimulatorRecordingPreferencesRecord, SimulatorSampleRecord, SimulatorSessionRecord, Store,
};

use crate::SimulatorTelemetryObserver;

const DEFAULT_RETENTION_DAYS: u32 = 30;
const MIN_RETENTION_DAYS: u32 = 1;
const MAX_RETENTION_DAYS: u32 = 3_650;
const MAX_SESSION_LIST: u32 = 50;
const MAX_GRAPH_SAMPLES: u32 = 600;
const GAP_AFTER_SECONDS: i64 = 3;

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
}

#[derive(Clone)]
pub struct SimulatorRecordingService {
    inner: Arc<SimulatorRecordingInner>,
}

struct SimulatorRecordingInner {
    store: Store,
    active: Mutex<Option<ActiveRecording>>,
    last_code: Mutex<Option<String>>,
}

struct ActiveRecording {
    id: String,
    provider_id: String,
    aircraft_title: String,
    aircraft_registration: Option<String>,
    last_sequence: u64,
    last_observed_at: DateTime<Utc>,
}

impl SimulatorRecordingService {
    pub fn new(store: Store) -> Self {
        let service = Self {
            inner: Arc::new(SimulatorRecordingInner {
                store,
                active: Mutex::new(None),
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
            .store
            .load_simulator_recording_preferences_record()
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)
            .map(|stored| SimulatorRecordingPreferences {
                retention_days: stored
                    .map(|preferences| preferences.retention_days)
                    .unwrap_or(DEFAULT_RETENTION_DAYS),
            })
    }

    pub fn update_preferences(
        &self,
        preferences: SimulatorRecordingPreferences,
    ) -> Result<SimulatorRecordingPreferences, SimulatorRecordingError> {
        if !(MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(&preferences.retention_days) {
            return Err(SimulatorRecordingError::InvalidRetention);
        }
        self.inner
            .store
            .save_simulator_recording_preferences_record(&SimulatorRecordingPreferencesRecord {
                retention_days: preferences.retention_days,
            })
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
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

        let id = Uuid::new_v4().to_string();
        let started_at = snapshot.provenance.retrieved_at;
        self.inner
            .store
            .create_simulator_session_record(&SimulatorSessionRecord {
                id: id.clone(),
                provider_id: provider_id.to_owned(),
                simulator_family: snapshot.simulator.family.clone(),
                simulator_version: snapshot.simulator.version.clone(),
                aircraft_title: snapshot.aircraft.title.clone(),
                aircraft_registration: snapshot.aircraft.registration.clone(),
                started_at: timestamp(started_at),
                ended_at: None,
                origin: "manual".into(),
                status: "active".into(),
                sample_count: 0,
            })
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?;
        if self
            .inner
            .store
            .append_simulator_sample_record(&id, &sample_from_snapshot(&snapshot, false))
            .is_err()
        {
            let _ = self.inner.store.finish_simulator_session_record(
                &id,
                &timestamp(Utc::now()),
                "interrupted",
            );
            return Err(SimulatorRecordingError::StorageUnavailable);
        }
        *active = Some(ActiveRecording {
            id,
            provider_id: provider_id.to_owned(),
            aircraft_title: snapshot.aircraft.title,
            aircraft_registration: snapshot.aircraft.registration,
            last_sequence: snapshot.sequence,
            last_observed_at: started_at,
        });
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
        *active = None;
        drop(active);
        self.clear_last_code()?;
        self.status()
    }

    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<SimulatorSessionView, SimulatorRecordingError> {
        let session = self
            .inner
            .store
            .list_simulator_session_records(MAX_SESSION_LIST)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or(SimulatorRecordingError::UnknownSession)
            .and_then(session_from_record)?;
        let samples = self
            .inner
            .store
            .list_simulator_sample_records(session_id, MAX_GRAPH_SAMPLES)
            .map_err(|_| SimulatorRecordingError::StorageUnavailable)?
            .into_iter()
            .map(sample_from_record)
            .collect();
        Ok(SimulatorSessionView {
            session,
            samples,
            sample_window_limit: MAX_GRAPH_SAMPLES,
        })
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
        let gap_before = snapshot.sequence != recording.last_sequence.saturating_add(1)
            || snapshot
                .provenance
                .retrieved_at
                .signed_duration_since(recording.last_observed_at)
                .num_seconds()
                > GAP_AFTER_SECONDS;
        match self.inner.store.append_simulator_sample_record(
            &recording.id,
            &sample_from_snapshot(snapshot, gap_before),
        ) {
            Ok(_) => {
                recording.last_sequence = snapshot.sequence;
                recording.last_observed_at = snapshot.provenance.retrieved_at;
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
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
#[path = "tests/simulator_recording.rs"]
mod tests;

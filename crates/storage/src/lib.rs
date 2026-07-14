//! Local-first SQLite storage and migration ownership.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial.sql");
const LEGAL_PREFERENCES_SCHEMA: &str = include_str!("../migrations/0002_legal_preferences.sql");
const THEMES_SCHEMA: &str = include_str!("../migrations/0003_themes.sql");
const PLUGIN_PERMISSIONS_SCHEMA: &str = include_str!("../migrations/0004_plugin_permissions.sql");
const LANGUAGE_PACKS_SCHEMA: &str = include_str!("../migrations/0005_language_packs.sql");
const DISPLAY_PREFERENCES_SCHEMA: &str = include_str!("../migrations/0006_display_preferences.sql");
const SIMULATOR_PREFERENCES_SCHEMA: &str =
    include_str!("../migrations/0007_simulator_preferences.sql");
const SIMULATOR_RECORDINGS_SCHEMA: &str =
    include_str!("../migrations/0008_simulator_recordings.sql");
const AUTHORIZATION_SCHEMA: &str = include_str!("../migrations/0009_authorization.sql");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Local storage state is unavailable")]
    StateUnavailable,
    #[error("Local storage record is outside supported bounds")]
    InvalidRecord,
}

#[derive(Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
    persistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiSnapshotRecord {
    pub resource_key: String,
    pub observed_at: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalPreferencesRecord {
    pub terms_version: String,
    pub privacy_notice_version: String,
    pub telemetry_enabled: bool,
    pub acknowledged_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreferencesRecord {
    pub selected_theme_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayPreferencesRecord {
    pub altitude_unit: String,
    pub speed_unit: String,
    pub weight_unit: String,
    pub fuel_unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorPreferencesRecord {
    pub selected_provider_id: Option<String>,
    pub start_with_wyrmgrid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorRecordingPreferencesRecord {
    pub retention_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorSessionRecord {
    pub id: String,
    pub provider_id: String,
    pub simulator_family: String,
    pub simulator_version: Option<String>,
    pub aircraft_title: String,
    pub aircraft_registration: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub origin: String,
    pub status: String,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulatorSampleRecord {
    pub source_sequence: u64,
    pub observed_at: String,
    pub simulation_time_utc: Option<String>,
    pub altitude_feet: f64,
    pub indicated_airspeed_knots: f64,
    pub true_airspeed_knots: f64,
    pub ground_speed_knots: f64,
    pub fuel_total_weight_pounds: Option<f64>,
    pub gross_weight_pounds: Option<f64>,
    pub pitch_degrees: f64,
    pub bank_degrees: f64,
    pub gap_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomThemeRecord {
    pub theme_id: String,
    pub manifest_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagePreferencesRecord {
    pub selected_language_pack_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomLanguagePackRecord {
    pub pack_id: String,
    pub manifest_json: String,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::configure_and_migrate(connection, true)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        Self::configure_and_migrate(connection, false)
    }

    fn configure_and_migrate(
        connection: Connection,
        persistent: bool,
    ) -> Result<Self, StorageError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )?;
        connection.execute_batch(INITIAL_SCHEMA)?;
        connection.execute_batch(LEGAL_PREFERENCES_SCHEMA)?;
        connection.execute_batch(THEMES_SCHEMA)?;
        connection.execute_batch(PLUGIN_PERMISSIONS_SCHEMA)?;
        connection.execute_batch(LANGUAGE_PACKS_SCHEMA)?;
        connection.execute_batch(DISPLAY_PREFERENCES_SCHEMA)?;
        connection.execute_batch(SIMULATOR_PREFERENCES_SCHEMA)?;
        connection.execute_batch(SIMULATOR_RECORDINGS_SCHEMA)?;
        connection.execute_batch(AUTHORIZATION_SCHEMA)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            persistent,
        })
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub fn load_display_preferences_record(
        &self,
    ) -> Result<Option<DisplayPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT altitude_unit, speed_unit, weight_unit, fuel_unit
                 FROM display_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(DisplayPreferencesRecord {
                        altitude_unit: row.get(0)?,
                        speed_unit: row.get(1)?,
                        weight_unit: row.get(2)?,
                        fuel_unit: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_display_preferences_record(
        &self,
        preferences: &DisplayPreferencesRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO display_preferences (
                    singleton_id, altitude_unit, speed_unit, weight_unit, fuel_unit
                 ) VALUES (1, ?1, ?2, ?3, ?4)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    altitude_unit = excluded.altitude_unit,
                    speed_unit = excluded.speed_unit,
                    weight_unit = excluded.weight_unit,
                    fuel_unit = excluded.fuel_unit,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    preferences.altitude_unit,
                    preferences.speed_unit,
                    preferences.weight_unit,
                    preferences.fuel_unit,
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn load_simulator_preferences_record(
        &self,
    ) -> Result<Option<SimulatorPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT selected_provider_id, start_with_wyrmgrid
                 FROM simulator_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(SimulatorPreferencesRecord {
                        selected_provider_id: row.get(0)?,
                        start_with_wyrmgrid: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_simulator_preferences_record(
        &self,
        preferences: &SimulatorPreferencesRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO simulator_preferences (
                    singleton_id, selected_provider_id, start_with_wyrmgrid
                 ) VALUES (1, ?1, ?2)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    selected_provider_id = excluded.selected_provider_id,
                    start_with_wyrmgrid = excluded.start_with_wyrmgrid,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    preferences.selected_provider_id,
                    preferences.start_with_wyrmgrid,
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn load_simulator_recording_preferences_record(
        &self,
    ) -> Result<Option<SimulatorRecordingPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT retention_days FROM simulator_recording_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(SimulatorRecordingPreferencesRecord {
                        retention_days: row.get(0)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_simulator_recording_preferences_record(
        &self,
        preferences: &SimulatorRecordingPreferencesRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO simulator_recording_preferences (singleton_id, retention_days)
                 VALUES (1, ?1)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    retention_days = excluded.retention_days,
                    updated_at = CURRENT_TIMESTAMP",
                [preferences.retention_days],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn interrupt_active_simulator_sessions(&self, ended_at: &str) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "UPDATE simulator_sessions
                 SET status = 'interrupted', ended_at = ?1
                 WHERE status = 'active'",
                [ended_at],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn create_simulator_session_record(
        &self,
        session: &SimulatorSessionRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO simulator_sessions (
                    id, provider_id, simulator_family, simulator_version,
                    aircraft_title, aircraft_registration, started_at, ended_at,
                    origin, status
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session.id,
                    session.provider_id,
                    session.simulator_family,
                    session.simulator_version,
                    session.aircraft_title,
                    session.aircraft_registration,
                    session.started_at,
                    session.ended_at,
                    session.origin,
                    session.status,
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn append_simulator_sample_record(
        &self,
        session_id: &str,
        sample: &SimulatorSampleRecord,
    ) -> Result<bool, StorageError> {
        let source_sequence =
            i64::try_from(sample.source_sequence).map_err(|_| StorageError::InvalidRecord)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO simulator_samples (
                    session_id, source_sequence, observed_at, simulation_time_utc,
                    altitude_feet, indicated_airspeed_knots, true_airspeed_knots,
                    ground_speed_knots, fuel_total_weight_pounds, gross_weight_pounds,
                    pitch_degrees, bank_degrees, gap_before
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    session_id,
                    source_sequence,
                    sample.observed_at,
                    sample.simulation_time_utc,
                    sample.altitude_feet,
                    sample.indicated_airspeed_knots,
                    sample.true_airspeed_knots,
                    sample.ground_speed_knots,
                    sample.fuel_total_weight_pounds,
                    sample.gross_weight_pounds,
                    sample.pitch_degrees,
                    sample.bank_degrees,
                    i64::from(sample.gap_before),
                ],
            )
            .map(|changed| changed == 1)
            .map_err(StorageError::from)
    }

    pub fn finish_simulator_session_record(
        &self,
        session_id: &str,
        ended_at: &str,
        status: &str,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "UPDATE simulator_sessions SET ended_at = ?2, status = ?3
                 WHERE id = ?1 AND status = 'active'",
                params![session_id, ended_at, status],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn list_simulator_session_records(
        &self,
        limit: u32,
    ) -> Result<Vec<SimulatorSessionRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT s.id, s.provider_id, s.simulator_family, s.simulator_version,
                    s.aircraft_title, s.aircraft_registration, s.started_at, s.ended_at,
                    s.origin, s.status, COUNT(p.id)
             FROM simulator_sessions s
             LEFT JOIN simulator_samples p ON p.session_id = s.id
             GROUP BY s.id
             ORDER BY s.started_at DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], |row| {
            let sample_count = row.get::<_, i64>(10)?;
            Ok(SimulatorSessionRecord {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                simulator_family: row.get(2)?,
                simulator_version: row.get(3)?,
                aircraft_title: row.get(4)?,
                aircraft_registration: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
                origin: row.get(8)?,
                status: row.get(9)?,
                sample_count: u64::try_from(sample_count)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(10, sample_count))?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn list_simulator_sample_records(
        &self,
        session_id: &str,
        limit: u32,
    ) -> Result<Vec<SimulatorSampleRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT source_sequence, observed_at, simulation_time_utc, altitude_feet,
                    indicated_airspeed_knots, true_airspeed_knots, ground_speed_knots,
                    fuel_total_weight_pounds, gross_weight_pounds, pitch_degrees,
                    bank_degrees, gap_before
             FROM (
                SELECT id, source_sequence, observed_at, simulation_time_utc, altitude_feet,
                       indicated_airspeed_knots, true_airspeed_knots, ground_speed_knots,
                       fuel_total_weight_pounds, gross_weight_pounds, pitch_degrees,
                       bank_degrees, gap_before
                FROM simulator_samples WHERE session_id = ?1
                ORDER BY id DESC LIMIT ?2
             ) ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![session_id, limit], |row| {
            let source_sequence = row.get::<_, i64>(0)?;
            Ok(SimulatorSampleRecord {
                source_sequence: u64::try_from(source_sequence)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, source_sequence))?,
                observed_at: row.get(1)?,
                simulation_time_utc: row.get(2)?,
                altitude_feet: row.get(3)?,
                indicated_airspeed_knots: row.get(4)?,
                true_airspeed_knots: row.get(5)?,
                ground_speed_knots: row.get(6)?,
                fuel_total_weight_pounds: row.get(7)?,
                gross_weight_pounds: row.get(8)?,
                pitch_degrees: row.get(9)?,
                bank_degrees: row.get(10)?,
                gap_before: row.get(11)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn delete_simulator_session_record(&self, session_id: &str) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute("DELETE FROM simulator_sessions WHERE id = ?1", [session_id])
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn delete_all_simulator_session_records(&self) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute("DELETE FROM simulator_sessions", [])
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn prune_simulator_session_records(&self, before: &str) -> Result<u64, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "DELETE FROM simulator_sessions
                 WHERE status != 'active' AND COALESCE(ended_at, started_at) < ?1",
                [before],
            )
            .map(|count| count as u64)
            .map_err(StorageError::from)
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        self.connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(StorageError::from)
    }

    pub fn save_api_snapshot(
        &mut self,
        resource_kind: &str,
        resource_key: &str,
        observed_at: &str,
        payload_json: &str,
    ) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO api_snapshots
                (resource_kind, resource_key, observed_at, payload_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![resource_kind, resource_key, observed_at, payload_json],
        )?;
        transaction.execute(
            "DELETE FROM api_snapshots
             WHERE resource_kind = ?1
               AND id NOT IN (
                   SELECT id
                   FROM (
                       SELECT
                           id,
                           ROW_NUMBER() OVER (
                               PARTITION BY
                                   resource_key,
                                   CASE
                                       WHEN datetime(observed_at) >= datetime('now', '-7 days')
                                           THEN strftime('%Y-%m-%dT%H', observed_at)
                                       ELSE date(observed_at)
                                   END
                               ORDER BY datetime(observed_at) DESC, id DESC
                           ) AS retention_rank
                       FROM api_snapshots
                       WHERE resource_kind = ?1
                   )
                   WHERE retention_rank = 1
               )",
            [resource_kind],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn latest_api_snapshot(
        &self,
        resource_kind: &str,
        resource_key: Option<&str>,
    ) -> Result<Option<ApiSnapshotRecord>, StorageError> {
        let (query, key) = match resource_key {
            Some(key) => (
                "SELECT resource_key, observed_at, payload_json
                 FROM api_snapshots
                 WHERE resource_kind = ?1 AND resource_key = ?2
                 ORDER BY datetime(observed_at) DESC, id DESC
                 LIMIT 1",
                Some(key),
            ),
            None => (
                "SELECT resource_key, observed_at, payload_json
                 FROM api_snapshots
                 WHERE resource_kind = ?1
                 ORDER BY datetime(observed_at) DESC, id DESC
                 LIMIT 1",
                None,
            ),
        };

        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(query)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(ApiSnapshotRecord {
                resource_key: row.get(0)?,
                observed_at: row.get(1)?,
                payload_json: row.get(2)?,
            })
        };
        match key {
            Some(key) => statement
                .query_row(params![resource_kind, key], map_row)
                .optional()
                .map_err(StorageError::from),
            None => statement
                .query_row([resource_kind], map_row)
                .optional()
                .map_err(StorageError::from),
        }
    }

    pub fn api_snapshot_at_or_before(
        &self,
        resource_kind: &str,
        resource_key: &str,
        observed_at: &str,
    ) -> Result<Option<ApiSnapshotRecord>, StorageError> {
        self.api_snapshot_history_at_or_before(resource_kind, resource_key, observed_at, 1)
            .map(|mut snapshots| snapshots.pop())
    }

    pub fn api_snapshot_history(
        &self,
        resource_kind: &str,
        resource_key: &str,
        limit: usize,
    ) -> Result<Vec<ApiSnapshotRecord>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT resource_key, observed_at, payload_json
             FROM (
                 SELECT id, resource_key, observed_at, payload_json
                 FROM api_snapshots
                 WHERE resource_kind = ?1 AND resource_key = ?2
                 ORDER BY datetime(observed_at) DESC, id DESC
                 LIMIT ?3
             )
             ORDER BY datetime(observed_at) ASC, id ASC",
        )?;
        statement
            .query_map(params![resource_kind, resource_key, limit], |row| {
                Ok(ApiSnapshotRecord {
                    resource_key: row.get(0)?,
                    observed_at: row.get(1)?,
                    payload_json: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn api_snapshot_history_at_or_before(
        &self,
        resource_kind: &str,
        resource_key: &str,
        observed_at: &str,
        limit: usize,
    ) -> Result<Vec<ApiSnapshotRecord>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT resource_key, observed_at, payload_json
             FROM (
                 SELECT id, resource_key, observed_at, payload_json
                 FROM api_snapshots
                 WHERE resource_kind = ?1
                   AND resource_key = ?2
                   AND datetime(observed_at) <= datetime(?3)
                 ORDER BY datetime(observed_at) DESC, id DESC
                 LIMIT ?4
             )
             ORDER BY datetime(observed_at) ASC, id ASC",
        )?;
        statement
            .query_map(
                params![resource_kind, resource_key, observed_at, limit],
                |row| {
                    Ok(ApiSnapshotRecord {
                        resource_key: row.get(0)?,
                        observed_at: row.get(1)?,
                        payload_json: row.get(2)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }
}

impl Store {
    pub fn list_authorization_grant_records(
        &self,
        subject_kind: &str,
        subject_id: &str,
        scope_revision: &str,
    ) -> Result<Vec<String>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT capability
             FROM authorization_grants
             WHERE subject_kind = ?1 AND subject_id = ?2 AND scope_revision = ?3
             ORDER BY capability ASC",
        )?;
        statement
            .query_map(params![subject_kind, subject_id, scope_revision], |row| {
                row.get(0)
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn replace_authorization_grant_records(
        &self,
        subject_kind: &str,
        subject_id: &str,
        scope_revision: &str,
        capabilities: &[String],
    ) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM authorization_grants
             WHERE subject_kind = ?1 AND subject_id = ?2",
            params![subject_kind, subject_id],
        )?;
        for capability in capabilities {
            transaction.execute(
                "INSERT INTO authorization_grants (
                    subject_kind, subject_id, scope_revision, capability
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![subject_kind, subject_id, scope_revision, capability],
            )?;
        }
        transaction.execute(
            "INSERT INTO authorization_decisions (
                subject_kind, subject_id, scope_revision, decision, capability_count
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                subject_kind,
                subject_id,
                scope_revision,
                if capabilities.is_empty() {
                    "revoke"
                } else {
                    "grant"
                },
                capabilities.len() as i64,
            ],
        )?;
        transaction.execute(
            "DELETE FROM authorization_decisions
             WHERE id NOT IN (
                 SELECT id FROM authorization_decisions ORDER BY id DESC LIMIT 4096
             )",
            [],
        )?;
        transaction.commit().map_err(StorageError::from)
    }
}

impl Store {
    pub fn list_plugin_permission_records(
        &self,
        plugin_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT permission
             FROM plugin_permission_grants
             WHERE plugin_id = ?1
             ORDER BY permission ASC",
        )?;
        statement
            .query_map([plugin_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn replace_plugin_permission_records(
        &self,
        plugin_id: &str,
        permissions: &[String],
    ) -> Result<(), StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM plugin_permission_grants WHERE plugin_id = ?1",
            [plugin_id],
        )?;
        for permission in permissions {
            transaction.execute(
                "INSERT INTO plugin_permission_grants (plugin_id, permission)
                 VALUES (?1, ?2)",
                params![plugin_id, permission],
            )?;
        }
        transaction.commit().map_err(StorageError::from)
    }
}

impl Store {
    pub fn load_language_preferences_record(
        &self,
    ) -> Result<Option<LanguagePreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT selected_language_pack_id
                 FROM language_preferences
                 WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(LanguagePreferencesRecord {
                        selected_language_pack_id: row.get(0)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_selected_language_pack_record(&self, pack_id: &str) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO language_preferences (singleton_id, selected_language_pack_id)
                 VALUES (1, ?1)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    selected_language_pack_id = excluded.selected_language_pack_id,
                    updated_at = CURRENT_TIMESTAMP",
                [pack_id],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn list_custom_language_pack_records(
        &self,
    ) -> Result<Vec<CustomLanguagePackRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT pack_id, manifest_json FROM custom_language_packs ORDER BY pack_id ASC",
        )?;
        statement
            .query_map([], |row| {
                Ok(CustomLanguagePackRecord {
                    pack_id: row.get(0)?,
                    manifest_json: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn save_custom_language_pack_record(
        &self,
        pack_id: &str,
        manifest_json: &str,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO custom_language_packs (pack_id, manifest_json)
                 VALUES (?1, ?2)
                 ON CONFLICT(pack_id) DO UPDATE SET
                    manifest_json = excluded.manifest_json,
                    updated_at = CURRENT_TIMESTAMP",
                params![pack_id, manifest_json],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }
}

impl Store {
    pub fn load_legal_preferences_record(
        &self,
    ) -> Result<Option<LegalPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT terms_version, privacy_notice_version, telemetry_enabled, acknowledged_at
                 FROM legal_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(LegalPreferencesRecord {
                        terms_version: row.get(0)?,
                        privacy_notice_version: row.get(1)?,
                        telemetry_enabled: row.get(2)?,
                        acknowledged_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_legal_preferences_record(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO legal_preferences (
                    singleton_id, terms_version, privacy_notice_version, telemetry_enabled
                 ) VALUES (1, ?1, ?2, ?3)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    acknowledged_at = CASE
                        WHEN terms_version <> excluded.terms_version
                          OR privacy_notice_version <> excluded.privacy_notice_version
                        THEN CURRENT_TIMESTAMP
                        ELSE acknowledged_at
                    END,
                    terms_version = excluded.terms_version,
                    privacy_notice_version = excluded.privacy_notice_version,
                    telemetry_enabled = excluded.telemetry_enabled,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    terms_version,
                    privacy_notice_version,
                    i64::from(telemetry_enabled)
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }
}

impl Store {
    pub fn load_theme_preferences_record(
        &self,
    ) -> Result<Option<ThemePreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT selected_theme_id FROM theme_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(ThemePreferencesRecord {
                        selected_theme_id: row.get(0)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_selected_theme_record(&self, theme_id: &str) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO theme_preferences (singleton_id, selected_theme_id)
                 VALUES (1, ?1)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    selected_theme_id = excluded.selected_theme_id,
                    updated_at = CURRENT_TIMESTAMP",
                [theme_id],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn list_custom_theme_records(&self) -> Result<Vec<CustomThemeRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection
            .prepare("SELECT theme_id, manifest_json FROM custom_themes ORDER BY theme_id ASC")?;
        let records = statement
            .query_map([], |row| {
                Ok(CustomThemeRecord {
                    theme_id: row.get(0)?,
                    manifest_json: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn save_custom_theme_record(
        &self,
        theme_id: &str,
        manifest_json: &str,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO custom_themes (theme_id, manifest_json)
                 VALUES (?1, ?2)
                 ON CONFLICT(theme_id) DO UPDATE SET
                    manifest_json = excluded.manifest_json,
                    updated_at = CURRENT_TIMESTAMP",
                params![theme_id, manifest_json],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }
}

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;

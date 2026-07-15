//! Local-first SQLite storage and migration ownership.

mod data_protection;

pub use data_protection::{
    DatabaseKey, PORTABLE_BACKUP_FORMAT_VERSION, PortableBackupRecord, PortableRestoreRecord,
    encrypted_database_state_exists,
};

use std::path::{Path, PathBuf};
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
const SIMULATOR_EVIDENCE_SCHEMA: &str = include_str!("../migrations/0010_simulator_evidence.sql");
const PROVIDER_ACCOUNTS_SCHEMA: &str = include_str!("../migrations/0011_provider_accounts.sql");
pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 11;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Local storage state is unavailable")]
    StateUnavailable,
    #[error("Local storage record is outside supported bounds")]
    InvalidRecord,
    #[error("The database encryption key is invalid")]
    InvalidDatabaseKey,
    #[error("SQLCipher is unavailable in this build")]
    EncryptionUnavailable,
    #[error("This operation requires persistent encrypted storage")]
    PersistentStorageRequired,
    #[error("The portable backup is invalid, damaged, or uses an unsupported format")]
    InvalidPortableBackup,
    #[error("The selected backup destination already exists")]
    BackupDestinationExists,
    #[error("A portable restore cannot use the active database as its source")]
    RestoreSourceIsActiveDatabase,
    #[error("Local storage file operation failed")]
    FileOperation(#[source] std::io::Error),
}

#[derive(Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
    persistent: bool,
    path: Option<PathBuf>,
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
pub struct AuthorizationGrantRecord {
    pub subject_kind: String,
    pub subject_id: String,
    pub scope_revision: String,
    pub capability: String,
    pub granted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationDecisionRecord {
    pub id: i64,
    pub subject_kind: String,
    pub subject_id: String,
    pub scope_revision: String,
    pub decision: String,
    pub capability_count: u32,
    pub decided_at: String,
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
    pub automatic_start: bool,
    pub automatic_stop: bool,
    pub landing_settle_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnAirAccountPreferencesRecord {
    pub company_id: String,
    pub connect_on_start: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimBriefAccountPreferencesRecord {
    pub reference_kind: String,
    pub reference: String,
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
    pub pinned: bool,
    pub plan_snapshot_json: Option<String>,
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
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub on_ground: Option<bool>,
    pub engines_running: Option<bool>,
    pub parking_brake_set: Option<bool>,
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatorSessionEventRecord {
    pub id: i64,
    pub event_kind: String,
    pub observed_at: String,
    pub source_sequence: Option<u64>,
    pub evidence_json: String,
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
    pub fn open(path: impl AsRef<Path>, key: &DatabaseKey) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        data_protection::activate_pending_restore(&path, key)?;
        let connection = data_protection::open_encrypted_connection(&path, key)?;
        let store = Self::configure_and_migrate(connection, Some(path.clone()))?;
        data_protection::finish_pending_restore(&path)?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        Self::configure_and_migrate(connection, None)
    }

    fn configure_and_migrate(
        connection: Connection,
        path: Option<PathBuf>,
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
        connection.execute_batch(SIMULATOR_EVIDENCE_SCHEMA)?;
        connection.execute_batch(PROVIDER_ACCOUNTS_SCHEMA)?;
        if path.is_some() {
            data_protection::mark_wyrmgrid_database(&connection)?;
        }
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            persistent: path.is_some(),
            path,
        })
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub fn load_onair_account_preferences_record(
        &self,
    ) -> Result<Option<OnAirAccountPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT company_id, connect_on_start
                 FROM onair_account_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(OnAirAccountPreferencesRecord {
                        company_id: row.get(0)?,
                        connect_on_start: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_onair_account_preferences_record(
        &self,
        preferences: &OnAirAccountPreferencesRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO onair_account_preferences (
                    singleton_id, company_id, connect_on_start
                 ) VALUES (1, ?1, ?2)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    company_id = excluded.company_id,
                    connect_on_start = excluded.connect_on_start,
                    updated_at = CURRENT_TIMESTAMP",
                params![preferences.company_id, preferences.connect_on_start],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn delete_onair_account_preferences_record(&self) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "DELETE FROM onair_account_preferences WHERE singleton_id = 1",
                [],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn load_simbrief_account_preferences_record(
        &self,
    ) -> Result<Option<SimBriefAccountPreferencesRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .query_row(
                "SELECT reference_kind, reference
                 FROM simbrief_account_preferences WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(SimBriefAccountPreferencesRecord {
                        reference_kind: row.get(0)?,
                        reference: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn save_simbrief_account_preferences_record(
        &self,
        preferences: &SimBriefAccountPreferencesRecord,
    ) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO simbrief_account_preferences (
                    singleton_id, reference_kind, reference
                 ) VALUES (1, ?1, ?2)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    reference_kind = excluded.reference_kind,
                    reference = excluded.reference,
                    updated_at = CURRENT_TIMESTAMP",
                params![preferences.reference_kind, preferences.reference],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn delete_simbrief_account_preferences_record(&self) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "DELETE FROM simbrief_account_preferences WHERE singleton_id = 1",
                [],
            )
            .map(|_| ())
            .map_err(StorageError::from)
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
                "SELECT r.retention_days,
                        COALESCE(a.automatic_start, 0),
                        COALESCE(a.automatic_stop, 1),
                        COALESCE(a.landing_settle_seconds, 30)
                 FROM simulator_recording_preferences r
                 LEFT JOIN simulator_recording_automation_preferences a
                    ON a.singleton_id = r.singleton_id
                 WHERE r.singleton_id = 1",
                [],
                |row| {
                    Ok(SimulatorRecordingPreferencesRecord {
                        retention_days: row.get(0)?,
                        automatic_start: row.get(1)?,
                        automatic_stop: row.get(2)?,
                        landing_settle_seconds: row.get(3)?,
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
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO simulator_recording_preferences (singleton_id, retention_days)
                 VALUES (1, ?1)
                 ON CONFLICT(singleton_id) DO UPDATE SET
                    retention_days = excluded.retention_days,
                    updated_at = CURRENT_TIMESTAMP",
            [preferences.retention_days],
        )?;
        transaction.execute(
            "INSERT INTO simulator_recording_automation_preferences (
                singleton_id, automatic_start, automatic_stop, landing_settle_seconds
             ) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(singleton_id) DO UPDATE SET
                automatic_start = excluded.automatic_start,
                automatic_stop = excluded.automatic_stop,
                landing_settle_seconds = excluded.landing_settle_seconds,
                updated_at = CURRENT_TIMESTAMP",
            params![
                preferences.automatic_start,
                preferences.automatic_stop,
                preferences.landing_settle_seconds,
            ],
        )?;
        transaction.commit().map_err(StorageError::from)
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
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO simulator_sessions (
                    id, provider_id, simulator_family, simulator_version,
                    aircraft_title, aircraft_registration, started_at, ended_at,
                    origin, status
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'manual', ?9)",
            params![
                session.id,
                session.provider_id,
                session.simulator_family,
                session.simulator_version,
                session.aircraft_title,
                session.aircraft_registration,
                session.started_at,
                session.ended_at,
                session.status,
            ],
        )?;
        transaction.execute(
            "INSERT INTO simulator_session_metadata (
                session_id, capture_mode, pinned, plan_snapshot_json, correlation_version
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session.id,
                session.origin,
                session.pinned,
                session.plan_snapshot_json,
                session.plan_snapshot_json.as_ref().map(|_| 1_i64),
            ],
        )?;
        transaction.commit().map_err(StorageError::from)
    }

    pub fn append_simulator_sample_record(
        &self,
        session_id: &str,
        sample: &SimulatorSampleRecord,
    ) -> Result<bool, StorageError> {
        let source_sequence =
            i64::try_from(sample.source_sequence).map_err(|_| StorageError::InvalidRecord)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
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
        )? == 1;
        if let (Some(latitude), Some(longitude), Some(on_ground)) =
            (sample.latitude, sample.longitude, sample.on_ground)
        {
            transaction.execute(
                "INSERT OR REPLACE INTO simulator_sample_facts (
                    session_id, source_sequence, observed_at, latitude, longitude,
                    on_ground, engines_running, parking_brake_set, paused
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    session_id,
                    source_sequence,
                    sample.observed_at,
                    latitude,
                    longitude,
                    on_ground,
                    sample.engines_running,
                    sample.parking_brake_set,
                    sample.paused,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(changed)
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
                    COALESCE(m.capture_mode, s.origin), s.status, COUNT(p.id),
                    COALESCE(m.pinned, 0), m.plan_snapshot_json
             FROM simulator_sessions s
             LEFT JOIN simulator_samples p ON p.session_id = s.id
             LEFT JOIN simulator_session_metadata m ON m.session_id = s.id
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
                pinned: row.get(11)?,
                plan_snapshot_json: row.get(12)?,
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
        self.list_simulator_sample_record_window(session_id, 0, limit)
    }

    pub fn list_simulator_sample_record_window(
        &self,
        session_id: &str,
        offset: u32,
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
                    bank_degrees, gap_before, latitude, longitude, on_ground,
                    engines_running, parking_brake_set, paused
             FROM (
                SELECT p.id, p.source_sequence, p.observed_at, p.simulation_time_utc,
                       p.altitude_feet, p.indicated_airspeed_knots, p.true_airspeed_knots,
                       p.ground_speed_knots, p.fuel_total_weight_pounds,
                       p.gross_weight_pounds, p.pitch_degrees, p.bank_degrees,
                       p.gap_before, f.latitude, f.longitude, f.on_ground,
                       f.engines_running, f.parking_brake_set, f.paused
                FROM simulator_samples p
                LEFT JOIN simulator_sample_facts f
                  ON f.session_id = p.session_id
                 AND f.source_sequence = p.source_sequence
                 AND f.observed_at = p.observed_at
                WHERE p.session_id = ?1
                ORDER BY p.id DESC LIMIT ?2 OFFSET ?3
             ) ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![session_id, limit, offset], |row| {
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
                latitude: row.get(12)?,
                longitude: row.get(13)?,
                on_ground: row.get(14)?,
                engines_running: row.get(15)?,
                parking_brake_set: row.get(16)?,
                paused: row.get(17)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn set_simulator_session_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<bool, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "UPDATE simulator_session_metadata
                 SET pinned = ?2, updated_at = CURRENT_TIMESTAMP
                 WHERE session_id = ?1",
                params![session_id, pinned],
            )
            .map(|changed| changed == 1)
            .map_err(StorageError::from)
    }

    pub fn attach_simulator_session_plan(
        &self,
        session_id: &str,
        plan_snapshot_json: &str,
        correlation_version: u32,
    ) -> Result<bool, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "UPDATE simulator_session_metadata
                 SET plan_snapshot_json = ?2, correlation_version = ?3,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE session_id = ?1",
                params![session_id, plan_snapshot_json, correlation_version],
            )
            .map(|changed| changed == 1)
            .map_err(StorageError::from)
    }

    pub fn append_simulator_session_event_record(
        &self,
        session_id: &str,
        event: &SimulatorSessionEventRecord,
    ) -> Result<(), StorageError> {
        let source_sequence = event
            .source_sequence
            .map(i64::try_from)
            .transpose()
            .map_err(|_| StorageError::InvalidRecord)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        connection
            .execute(
                "INSERT INTO simulator_session_events (
                    session_id, event_kind, observed_at, source_sequence, evidence_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session_id,
                    event.event_kind,
                    event.observed_at,
                    source_sequence,
                    event.evidence_json,
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from)
    }

    pub fn list_simulator_session_event_records(
        &self,
        session_id: &str,
    ) -> Result<Vec<SimulatorSessionEventRecord>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT id, event_kind, observed_at, source_sequence, evidence_json
             FROM simulator_session_events
             WHERE session_id = ?1
             ORDER BY id ASC",
        )?;
        statement
            .query_map([session_id], |row| {
                let source_sequence = row
                    .get::<_, Option<i64>>(3)?
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, -1))?;
                Ok(SimulatorSessionEventRecord {
                    id: row.get(0)?,
                    event_kind: row.get(1)?,
                    observed_at: row.get(2)?,
                    source_sequence,
                    evidence_json: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
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
                 WHERE status != 'active' AND COALESCE(ended_at, started_at) < ?1
                   AND NOT EXISTS (
                       SELECT 1 FROM simulator_session_metadata m
                       WHERE m.session_id = simulator_sessions.id AND m.pinned = 1
                   )",
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
    pub fn list_active_authorization_grant_records(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationGrantRecord>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT subject_kind, subject_id, scope_revision, capability, granted_at
             FROM authorization_grants
             ORDER BY subject_kind ASC, subject_id ASC, scope_revision ASC, capability ASC
             LIMIT ?1",
        )?;
        statement
            .query_map([limit], |row| {
                Ok(AuthorizationGrantRecord {
                    subject_kind: row.get(0)?,
                    subject_id: row.get(1)?,
                    scope_revision: row.get(2)?,
                    capability: row.get(3)?,
                    granted_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn list_authorization_decision_records(
        &self,
        limit: usize,
    ) -> Result<Vec<AuthorizationDecisionRecord>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::StateUnavailable)?;
        let mut statement = connection.prepare(
            "SELECT id, subject_kind, subject_id, scope_revision, decision,
                    capability_count, decided_at
             FROM authorization_decisions
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        statement
            .query_map([limit], |row| {
                let capability_count = row.get::<_, i64>(5)?;
                let capability_count = u32::try_from(capability_count)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, capability_count))?;
                Ok(AuthorizationDecisionRecord {
                    id: row.get(0)?,
                    subject_kind: row.get(1)?,
                    subject_id: row.get(2)?,
                    scope_revision: row.get(3)?,
                    decision: row.get(4)?,
                    capability_count,
                    decided_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

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

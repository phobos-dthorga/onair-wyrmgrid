//! Local-first SQLite storage and migration ownership.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial.sql");
const LEGAL_PREFERENCES_SCHEMA: &str = include_str!("../migrations/0002_legal_preferences.sql");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Local storage state is unavailable")]
    StateUnavailable,
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
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            persistent,
        })
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, SecondsFormat, Timelike, Utc};

    #[test]
    fn initializes_the_database_schema() {
        let store = Store::open_in_memory().expect("in-memory database should open");
        assert_eq!(
            store.schema_version().expect("version should be readable"),
            2
        );
    }

    #[test]
    fn stores_and_restores_the_latest_snapshot() {
        let mut store = Store::open_in_memory().expect("in-memory database should open");
        let earlier = (Utc::now() - Duration::hours(2)).to_rfc3339_opts(SecondsFormat::Secs, true);
        let latest = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        store
            .save_api_snapshot("fleet", "company-a", &earlier, "{\"version\":1}")
            .expect("earlier snapshot should save");
        store
            .save_api_snapshot("fleet", "company-a", &latest, "{\"version\":2}")
            .expect("latest snapshot should save");

        assert_eq!(
            store
                .latest_api_snapshot("fleet", Some("company-a"))
                .expect("snapshot lookup should succeed"),
            Some(ApiSnapshotRecord {
                resource_key: "company-a".into(),
                observed_at: latest,
                payload_json: "{\"version\":2}".into(),
            })
        );
    }

    #[test]
    fn retains_hourly_recent_and_daily_older_snapshots_per_company() {
        let mut store = Store::open_in_memory().expect("in-memory database should open");
        let now = Utc::now();
        let completed_hour = (now - Duration::hours(1))
            .with_minute(0)
            .and_then(|value| value.with_second(0))
            .and_then(|value| value.with_nanosecond(0))
            .expect("completed hour should be representable");
        let older_day = (now - Duration::days(8))
            .with_hour(0)
            .and_then(|value| value.with_minute(0))
            .and_then(|value| value.with_second(0))
            .and_then(|value| value.with_nanosecond(0))
            .expect("older day should be representable");
        let observations = [
            completed_hour + Duration::minutes(10),
            completed_hour + Duration::minutes(40),
            now - Duration::hours(2),
            older_day + Duration::hours(3),
            older_day + Duration::hours(8),
            now - Duration::days(9),
        ];

        for (index, observed_at) in observations.iter().enumerate() {
            store
                .save_api_snapshot(
                    "fleet",
                    "company-a",
                    &observed_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                    &format!("{{\"index\":{index}}}"),
                )
                .expect("snapshot should save");
        }
        store
            .save_api_snapshot(
                "fleet",
                "company-b",
                &now.to_rfc3339_opts(SecondsFormat::Secs, true),
                "{\"company\":\"b\"}",
            )
            .expect("other company snapshot should save");

        let connection = store
            .connection
            .lock()
            .expect("storage connection should be available");
        let company_a_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM api_snapshots
                 WHERE resource_kind = 'fleet' AND resource_key = 'company-a'",
                [],
                |row| row.get(0),
            )
            .expect("retained count should be available");
        let company_b_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM api_snapshots
                 WHERE resource_kind = 'fleet' AND resource_key = 'company-b'",
                [],
                |row| row.get(0),
            )
            .expect("other company count should be available");

        assert_eq!(company_a_count, 4);
        assert_eq!(company_b_count, 1);
    }

    #[test]
    fn persists_legal_acknowledgement_and_telemetry_choice() {
        let store = Store::open_in_memory().expect("in-memory database should open");
        assert!(
            store
                .load_legal_preferences_record()
                .expect("preferences should be readable")
                .is_none()
        );

        store
            .save_legal_preferences_record("terms-v1", "privacy-v1", true)
            .expect("preferences should be saved");
        let preferences = store
            .load_legal_preferences_record()
            .expect("preferences should be readable")
            .expect("preferences should exist");
        assert_eq!(preferences.terms_version, "terms-v1");
        assert_eq!(preferences.privacy_notice_version, "privacy-v1");
        assert!(preferences.telemetry_enabled);
        assert!(!preferences.acknowledged_at.is_empty());
    }
}

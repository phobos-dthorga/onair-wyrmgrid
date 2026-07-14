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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreferencesRecord {
    pub selected_theme_id: String,
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

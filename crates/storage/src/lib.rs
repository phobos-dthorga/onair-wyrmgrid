//! Local-first SQLite storage and migration ownership.

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::configure_and_migrate(connection)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        Self::configure_and_migrate(connection)
    }

    fn configure_and_migrate(connection: Connection) -> Result<Self, StorageError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )?;
        connection.execute_batch(INITIAL_SCHEMA)?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        self.connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(StorageError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_the_database_schema() {
        let store = Store::open_in_memory().expect("in-memory database should open");
        assert_eq!(
            store.schema_version().expect("version should be readable"),
            1
        );
    }
}

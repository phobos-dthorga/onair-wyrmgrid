CREATE TABLE IF NOT EXISTS flight_operations (
    operation_id TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,
    current_revision INTEGER NOT NULL CHECK (current_revision > 0),
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS flight_operation_revisions (
    operation_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0),
    reason TEXT NOT NULL CHECK (
        reason IN ('initial', 'plan_changed', 'job_changed', 'plan_and_job_changed')
    ),
    created_at TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    PRIMARY KEY (operation_id, revision),
    FOREIGN KEY (operation_id) REFERENCES flight_operations(operation_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS active_flight_operation (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    operation_id TEXT NOT NULL,
    FOREIGN KEY (operation_id) REFERENCES flight_operations(operation_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_flight_operation_revisions_created
    ON flight_operation_revisions(operation_id, created_at, revision);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (13);

PRAGMA user_version = 13;

CREATE TABLE IF NOT EXISTS flight_operation_aircraft_assignment_revisions (
    operation_id TEXT NOT NULL,
    assignment_revision INTEGER NOT NULL CHECK (assignment_revision > 0),
    reason TEXT NOT NULL CHECK (reason IN ('assigned', 'reassigned', 'cleared')),
    reviewed_at TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    PRIMARY KEY (operation_id, assignment_revision),
    FOREIGN KEY (operation_id) REFERENCES flight_operations(operation_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_flight_operation_aircraft_assignments_reviewed
    ON flight_operation_aircraft_assignment_revisions(operation_id, reviewed_at, assignment_revision);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (19);

PRAGMA user_version = 19;

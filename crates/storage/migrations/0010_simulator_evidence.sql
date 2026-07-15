CREATE TABLE IF NOT EXISTS simulator_recording_automation_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    automatic_start INTEGER NOT NULL DEFAULT 0 CHECK (automatic_start IN (0, 1)),
    automatic_stop INTEGER NOT NULL DEFAULT 1 CHECK (automatic_stop IN (0, 1)),
    landing_settle_seconds INTEGER NOT NULL DEFAULT 30
        CHECK (landing_settle_seconds BETWEEN 10 AND 600),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS simulator_session_metadata (
    session_id TEXT PRIMARY KEY REFERENCES simulator_sessions(id) ON DELETE CASCADE,
    capture_mode TEXT NOT NULL DEFAULT 'manual'
        CHECK (capture_mode IN ('manual', 'automatic')),
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    plan_snapshot_json TEXT,
    correlation_version INTEGER,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_simulator_session_metadata_pinned
    ON simulator_session_metadata(pinned, session_id);

INSERT OR IGNORE INTO simulator_session_metadata (session_id, capture_mode)
SELECT id, 'manual' FROM simulator_sessions;

CREATE TABLE IF NOT EXISTS simulator_sample_facts (
    session_id TEXT NOT NULL REFERENCES simulator_sessions(id) ON DELETE CASCADE,
    source_sequence INTEGER NOT NULL,
    observed_at TEXT NOT NULL,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    on_ground INTEGER NOT NULL CHECK (on_ground IN (0, 1)),
    engines_running INTEGER CHECK (engines_running IN (0, 1)),
    parking_brake_set INTEGER CHECK (parking_brake_set IN (0, 1)),
    paused INTEGER CHECK (paused IN (0, 1)),
    PRIMARY KEY (session_id, source_sequence, observed_at)
);

CREATE TABLE IF NOT EXISTS simulator_session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES simulator_sessions(id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    source_sequence INTEGER,
    evidence_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_simulator_session_events_session
    ON simulator_session_events(session_id, id);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (10);

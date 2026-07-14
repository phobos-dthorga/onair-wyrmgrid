CREATE TABLE IF NOT EXISTS simulator_recording_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    retention_days INTEGER NOT NULL DEFAULT 30 CHECK (retention_days BETWEEN 1 AND 3650),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS simulator_sessions (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    simulator_family TEXT NOT NULL,
    simulator_version TEXT,
    aircraft_title TEXT NOT NULL,
    aircraft_registration TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    origin TEXT NOT NULL CHECK (origin IN ('manual')),
    status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'interrupted'))
);

CREATE INDEX IF NOT EXISTS idx_simulator_sessions_started
    ON simulator_sessions(started_at DESC);

CREATE TABLE IF NOT EXISTS simulator_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES simulator_sessions(id) ON DELETE CASCADE,
    source_sequence INTEGER NOT NULL,
    observed_at TEXT NOT NULL,
    simulation_time_utc TEXT,
    altitude_feet REAL NOT NULL,
    indicated_airspeed_knots REAL NOT NULL,
    true_airspeed_knots REAL NOT NULL,
    ground_speed_knots REAL NOT NULL,
    fuel_total_weight_pounds REAL,
    gross_weight_pounds REAL,
    pitch_degrees REAL NOT NULL,
    bank_degrees REAL NOT NULL,
    gap_before INTEGER NOT NULL DEFAULT 0 CHECK (gap_before IN (0, 1)),
    UNIQUE(session_id, source_sequence, observed_at)
);

CREATE INDEX IF NOT EXISTS idx_simulator_samples_session
    ON simulator_samples(session_id, id);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (8);

CREATE TABLE IF NOT EXISTS legal_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    terms_version TEXT NOT NULL,
    privacy_notice_version TEXT NOT NULL,
    telemetry_enabled INTEGER NOT NULL CHECK (telemetry_enabled IN (0, 1)),
    acknowledged_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (2);

CREATE TABLE IF NOT EXISTS simulator_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_provider_id TEXT,
    start_with_wyrmgrid INTEGER NOT NULL DEFAULT 0 CHECK (start_with_wyrmgrid IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (7);

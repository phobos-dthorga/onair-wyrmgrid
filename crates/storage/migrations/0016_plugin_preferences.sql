CREATE TABLE IF NOT EXISTS plugin_preferences (
    plugin_id TEXT PRIMARY KEY NOT NULL,
    scope_revision TEXT NOT NULL,
    start_with_wyrmgrid INTEGER NOT NULL DEFAULT 0
        CHECK (start_with_wyrmgrid IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (16);

PRAGMA user_version = 16;

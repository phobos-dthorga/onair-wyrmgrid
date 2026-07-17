CREATE TABLE IF NOT EXISTS interaction_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    responsive_surfaces INTEGER NOT NULL DEFAULT 1 CHECK (responsive_surfaces IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (12);

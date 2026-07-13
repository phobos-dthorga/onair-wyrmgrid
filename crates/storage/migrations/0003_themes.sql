CREATE TABLE IF NOT EXISTS custom_themes (
    theme_id TEXT PRIMARY KEY,
    manifest_json TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS theme_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_theme_id TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (3);

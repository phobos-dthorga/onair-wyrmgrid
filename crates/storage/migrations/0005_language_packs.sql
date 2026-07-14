CREATE TABLE IF NOT EXISTS custom_language_packs (
    pack_id TEXT PRIMARY KEY,
    manifest_json TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS language_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_language_pack_id TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (5);

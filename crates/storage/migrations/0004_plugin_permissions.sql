CREATE TABLE IF NOT EXISTS plugin_permission_grants (
    plugin_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (plugin_id, permission)
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (4);

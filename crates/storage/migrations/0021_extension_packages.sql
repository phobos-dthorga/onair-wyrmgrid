CREATE TABLE IF NOT EXISTS extension_package_versions (
    package_kind TEXT NOT NULL
        CHECK (package_kind IN ('ordinary_plugin', 'simulator_provider', 'audio_provider')),
    extension_id TEXT NOT NULL,
    version TEXT NOT NULL,
    archive_sha256 TEXT NOT NULL
        CHECK (length(archive_sha256) = 64),
    package_schema_version INTEGER NOT NULL
        CHECK (package_schema_version > 0),
    source TEXT NOT NULL
        CHECK (source IN ('local_file', 'first_party')),
    package_manifest_json TEXT NOT NULL,
    extension_manifest_json TEXT NOT NULL,
    installed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (package_kind, extension_id, version)
);

CREATE TABLE IF NOT EXISTS extension_package_state (
    package_kind TEXT NOT NULL,
    extension_id TEXT NOT NULL,
    active_version TEXT NOT NULL,
    rollback_version TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
        CHECK (enabled IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (package_kind, extension_id),
    FOREIGN KEY (package_kind, extension_id, active_version)
        REFERENCES extension_package_versions (package_kind, extension_id, version)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (package_kind, extension_id, rollback_version)
        REFERENCES extension_package_versions (package_kind, extension_id, version)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_extension_package_versions_installed_at
    ON extension_package_versions (package_kind, extension_id, installed_at DESC);

CREATE TABLE IF NOT EXISTS audio_provider_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    selected_provider_id TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (21);

PRAGMA user_version = 21;
INSERT OR IGNORE INTO schema_migrations (version) VALUES (20);

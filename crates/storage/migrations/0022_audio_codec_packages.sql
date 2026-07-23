CREATE TABLE IF NOT EXISTS extension_package_versions_v22 (
    package_kind TEXT NOT NULL
        CHECK (package_kind IN (
            'ordinary_plugin',
            'simulator_provider',
            'audio_provider',
            'audio_codec_provider'
        )),
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

CREATE TABLE IF NOT EXISTS extension_package_state_v22 (
    package_kind TEXT NOT NULL,
    extension_id TEXT NOT NULL,
    active_version TEXT NOT NULL,
    rollback_version TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
        CHECK (enabled IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (package_kind, extension_id),
    FOREIGN KEY (package_kind, extension_id, active_version)
        REFERENCES extension_package_versions_v22 (package_kind, extension_id, version)
        ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (package_kind, extension_id, rollback_version)
        REFERENCES extension_package_versions_v22 (package_kind, extension_id, version)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

INSERT INTO extension_package_versions_v22 (
    package_kind,
    extension_id,
    version,
    archive_sha256,
    package_schema_version,
    source,
    package_manifest_json,
    extension_manifest_json,
    installed_at
)
SELECT
    package_kind,
    extension_id,
    version,
    archive_sha256,
    package_schema_version,
    source,
    package_manifest_json,
    extension_manifest_json,
    installed_at
FROM extension_package_versions;

INSERT INTO extension_package_state_v22 (
    package_kind,
    extension_id,
    active_version,
    rollback_version,
    enabled,
    updated_at
)
SELECT
    package_kind,
    extension_id,
    active_version,
    rollback_version,
    enabled,
    updated_at
FROM extension_package_state;

DROP TABLE extension_package_state;
DROP TABLE extension_package_versions;

ALTER TABLE extension_package_versions_v22
    RENAME TO extension_package_versions;
ALTER TABLE extension_package_state_v22
    RENAME TO extension_package_state;

CREATE INDEX IF NOT EXISTS idx_extension_package_versions_installed_at
    ON extension_package_versions (package_kind, extension_id, installed_at DESC);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (22);

PRAGMA user_version = 22;

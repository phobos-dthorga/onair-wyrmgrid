CREATE TABLE IF NOT EXISTS authorization_grants (
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    scope_revision TEXT NOT NULL,
    capability TEXT NOT NULL,
    granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subject_kind, subject_id, capability)
);

CREATE INDEX IF NOT EXISTS idx_authorization_grants_subject
    ON authorization_grants (subject_kind, subject_id, scope_revision);

CREATE TABLE IF NOT EXISTS authorization_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    scope_revision TEXT NOT NULL,
    decision TEXT NOT NULL CHECK (decision IN ('grant', 'revoke')),
    capability_count INTEGER NOT NULL CHECK (capability_count >= 0),
    decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_authorization_decisions_subject
    ON authorization_decisions (subject_kind, subject_id, decided_at DESC);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (9);

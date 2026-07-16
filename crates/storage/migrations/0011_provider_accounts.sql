CREATE TABLE IF NOT EXISTS onair_account_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    company_id TEXT NOT NULL,
    connect_on_start INTEGER NOT NULL DEFAULT 0 CHECK (connect_on_start IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS simbrief_account_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    reference_kind TEXT NOT NULL CHECK (reference_kind IN ('pilot_id', 'username')),
    reference TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (11);

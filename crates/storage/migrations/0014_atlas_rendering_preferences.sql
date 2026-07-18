CREATE TABLE IF NOT EXISTS atlas_rendering_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    weather_rendering_profile TEXT NOT NULL DEFAULT 'enhanced'
        CHECK (weather_rendering_profile IN ('compatibility', 'enhanced')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (14);

CREATE TABLE IF NOT EXISTS atlas_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    automatic_sync_minutes INTEGER NOT NULL DEFAULT 30
        CHECK (automatic_sync_minutes IN (0, 15, 30, 60, 120)),
    daylight_visible INTEGER NOT NULL DEFAULT 1 CHECK (daylight_visible IN (0, 1)),
    regions_visible INTEGER NOT NULL DEFAULT 1 CHECK (regions_visible IN (0, 1)),
    route_visible INTEGER NOT NULL DEFAULT 1 CHECK (route_visible IN (0, 1)),
    fleet_visible INTEGER NOT NULL DEFAULT 1 CHECK (fleet_visible IN (0, 1)),
    fbos_visible INTEGER NOT NULL DEFAULT 1 CHECK (fbos_visible IN (0, 1)),
    airport_weather_visible INTEGER NOT NULL DEFAULT 1
        CHECK (airport_weather_visible IN (0, 1)),
    global_weather_visible INTEGER NOT NULL DEFAULT 1
        CHECK (global_weather_visible IN (0, 1)),
    weather_coverage_visible INTEGER NOT NULL DEFAULT 1
        CHECK (weather_coverage_visible IN (0, 1)),
    plugin_layers_visible INTEGER NOT NULL DEFAULT 1
        CHECK (plugin_layers_visible IN (0, 1)),
    restore_last_view INTEGER NOT NULL DEFAULT 0 CHECK (restore_last_view IN (0, 1)),
    last_longitude REAL,
    last_latitude REAL,
    last_zoom REAL,
    last_bearing REAL,
    last_pitch REAL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        (last_longitude IS NULL AND last_latitude IS NULL AND last_zoom IS NULL
            AND last_bearing IS NULL AND last_pitch IS NULL)
        OR
        (last_longitude IS NOT NULL AND last_latitude IS NOT NULL
            AND last_zoom IS NOT NULL AND last_bearing IS NOT NULL
            AND last_pitch IS NOT NULL
            AND last_longitude BETWEEN -180.0 AND 180.0
            AND last_latitude BETWEEN -90.0 AND 90.0
            AND last_zoom BETWEEN 0.0 AND 24.0
            AND last_bearing BETWEEN -180.0 AND 180.0
            AND last_pitch BETWEEN 0.0 AND 85.0)
    ),
    CHECK (
        restore_last_view = 1
        OR (last_longitude IS NULL AND last_latitude IS NULL AND last_zoom IS NULL
            AND last_bearing IS NULL AND last_pitch IS NULL)
    )
);

CREATE TABLE IF NOT EXISTS plugin_configuration (
    plugin_id TEXT NOT NULL CHECK (length(plugin_id) BETWEEN 1 AND 128),
    setting_key TEXT NOT NULL CHECK (length(setting_key) BETWEEN 1 AND 64),
    value TEXT NOT NULL CHECK (length(value) BETWEEN 1 AND 256),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (plugin_id, setting_key)
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (17);

PRAGMA user_version = 17;

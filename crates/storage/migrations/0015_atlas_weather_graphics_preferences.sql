CREATE TABLE IF NOT EXISTS atlas_weather_graphics_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    weather_rendering_profile TEXT NOT NULL DEFAULT 'enhanced'
        CHECK (weather_rendering_profile IN ('compatibility', 'enhanced', 'cinematic')),
    weather_cloud_effects INTEGER NOT NULL DEFAULT 1
        CHECK (weather_cloud_effects IN (0, 1)),
    weather_precipitation_effects INTEGER NOT NULL DEFAULT 1
        CHECK (weather_precipitation_effects IN (0, 1)),
    weather_lightning_effects INTEGER NOT NULL DEFAULT 1
        CHECK (weather_lightning_effects IN (0, 1)),
    weather_dust_effects INTEGER NOT NULL DEFAULT 1
        CHECK (weather_dust_effects IN (0, 1)),
    reduce_weather_flashes INTEGER NOT NULL DEFAULT 1
        CHECK (reduce_weather_flashes IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Migration 0014 remains the legacy two-profile fallback. Seed this richer,
-- authoritative record once without rewriting the shipped table.
INSERT OR IGNORE INTO atlas_weather_graphics_preferences (
    singleton_id,
    weather_rendering_profile,
    weather_cloud_effects,
    weather_precipitation_effects,
    weather_lightning_effects,
    weather_dust_effects,
    reduce_weather_flashes
)
SELECT
    singleton_id,
    weather_rendering_profile,
    1,
    1,
    1,
    1,
    1
FROM atlas_rendering_preferences;

INSERT OR IGNORE INTO schema_migrations (version) VALUES (15);

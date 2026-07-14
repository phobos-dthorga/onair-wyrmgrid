CREATE TABLE IF NOT EXISTS display_preferences (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    altitude_unit TEXT NOT NULL CHECK (altitude_unit IN ('feet', 'metres')),
    speed_unit TEXT NOT NULL CHECK (speed_unit IN ('knots', 'miles_per_hour', 'kilometres_per_hour', 'metres_per_second')),
    weight_unit TEXT NOT NULL CHECK (weight_unit IN ('pounds', 'kilograms')),
    fuel_unit TEXT NOT NULL CHECK (fuel_unit IN ('pounds', 'kilograms', 'us_gallons', 'imperial_gallons', 'litres')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (6);

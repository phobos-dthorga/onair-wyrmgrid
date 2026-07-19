use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct MemoryDisplayPreferences {
    value: Mutex<Option<DisplayPreferences>>,
}

impl DisplayPreferencesRepository for MemoryDisplayPreferences {
    fn load_display_preferences(&self) -> Result<Option<DisplayPreferences>, DisplaySettingsError> {
        self.value
            .lock()
            .map(|value| *value)
            .map_err(|_| DisplaySettingsError::StorageUnavailable)
    }

    fn save_display_preferences(
        &self,
        preferences: DisplayPreferences,
    ) -> Result<(), DisplaySettingsError> {
        *self
            .value
            .lock()
            .map_err(|_| DisplaySettingsError::StorageUnavailable)? = Some(preferences);
        Ok(())
    }
}

#[test]
fn defaults_to_the_existing_aviation_presentation() {
    let service = DisplaySettingsService::new(MemoryDisplayPreferences::default());

    let preferences = service.status().unwrap();
    assert_eq!(preferences, DisplayPreferences::default());
    assert_eq!(
        preferences.weather_rendering_profile,
        WeatherRenderingProfile::Enhanced
    );
}

#[test]
fn persists_each_measurement_category_independently() {
    let service = DisplaySettingsService::new(MemoryDisplayPreferences::default());
    let preferences = DisplayPreferences {
        altitude_unit: AltitudeUnit::Metres,
        speed_unit: SpeedUnit::Knots,
        weight_unit: WeightUnit::Kilograms,
        fuel_unit: FuelUnit::Litres,
        responsive_surfaces: false,
        weather_rendering_profile: WeatherRenderingProfile::Cinematic,
        weather_cloud_effects: true,
        weather_precipitation_effects: false,
        weather_lightning_effects: true,
        weather_dust_effects: false,
        reduce_weather_flashes: false,
    };

    assert_eq!(service.update(preferences).unwrap(), preferences);
    assert_eq!(service.status().unwrap(), preferences);
}

#[test]
fn cinematic_weather_preferences_round_trip_through_storage_records() {
    let preferences = DisplayPreferences {
        weather_rendering_profile: WeatherRenderingProfile::Cinematic,
        weather_precipitation_effects: false,
        weather_dust_effects: false,
        reduce_weather_flashes: false,
        ..DisplayPreferences::default()
    };

    let record = preferences_to_record(preferences);
    assert_eq!(record.weather_rendering_profile, "cinematic");
    assert!(!record.weather_precipitation_effects);
    assert!(!record.weather_dust_effects);
    assert!(!record.reduce_weather_flashes);
    assert_eq!(record_to_preferences(record).unwrap(), preferences);
}

#[test]
fn unsupported_weather_profiles_fail_closed() {
    let mut record = preferences_to_record(DisplayPreferences::default());
    record.weather_rendering_profile = "unbounded".into();

    assert_eq!(
        record_to_preferences(record),
        Err(DisplaySettingsError::UnsupportedUnit)
    );
}

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
        weather_rendering_profile: WeatherRenderingProfile::Compatibility,
    };

    assert_eq!(service.update(preferences).unwrap(), preferences);
    assert_eq!(service.status().unwrap(), preferences);
}

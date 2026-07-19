use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct MemoryAtlasPreferences {
    value: Mutex<Option<AtlasPreferences>>,
}

impl AtlasPreferencesRepository for MemoryAtlasPreferences {
    fn load_atlas_preferences(&self) -> Result<Option<AtlasPreferences>, AtlasPreferencesError> {
        self.value
            .lock()
            .map(|value| *value)
            .map_err(|_| AtlasPreferencesError::StorageUnavailable)
    }

    fn save_atlas_preferences(
        &self,
        mut preferences: AtlasPreferences,
    ) -> Result<(), AtlasPreferencesError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| AtlasPreferencesError::StorageUnavailable)?;
        if !preferences.restore_last_view {
            preferences.last_view = None;
        }
        *value = Some(preferences);
        Ok(())
    }

    fn save_atlas_view(&self, view: AtlasView) -> Result<(), AtlasPreferencesError> {
        let mut value = self
            .value
            .lock()
            .map_err(|_| AtlasPreferencesError::StorageUnavailable)?;
        if let Some(preferences) = value.as_mut()
            && preferences.restore_last_view
        {
            preferences.last_view = Some(view);
        }
        Ok(())
    }
}

#[test]
fn defaults_preserve_current_atlas_behaviour_without_retaining_a_view() {
    let service = AtlasPreferencesService::new(MemoryAtlasPreferences::default());

    let preferences = service.status().unwrap();
    assert_eq!(preferences.automatic_sync_minutes, 30);
    assert_eq!(preferences.layers, AtlasLayerVisibility::default());
    assert!(!preferences.restore_last_view);
    assert_eq!(preferences.last_view, None);
}

#[test]
fn preferences_round_trip_and_disabling_restore_clears_the_view() {
    let service = AtlasPreferencesService::new(MemoryAtlasPreferences::default());
    let mut preferences = AtlasPreferences {
        automatic_sync_minutes: 60,
        layers: AtlasLayerVisibility {
            daylight: false,
            global_weather: false,
            ..AtlasLayerVisibility::default()
        },
        restore_last_view: true,
        last_view: None,
    };
    service.update(preferences).unwrap();
    let view = AtlasView {
        longitude: 151.2093,
        latitude: -33.8688,
        zoom: 6.5,
        bearing: 12.0,
        pitch: 35.0,
    };
    assert_eq!(service.update_view(view).unwrap().last_view, Some(view));

    preferences = service.status().unwrap();
    preferences.restore_last_view = false;
    let disabled = service.update(preferences).unwrap();
    assert_eq!(disabled.last_view, None);

    preferences.restore_last_view = true;
    assert_eq!(service.update(preferences).unwrap().last_view, None);
}

#[test]
fn unsupported_sync_intervals_are_rejected_without_overwriting_state() {
    let service = AtlasPreferencesService::new(MemoryAtlasPreferences::default());
    let invalid = AtlasPreferences {
        automatic_sync_minutes: 7,
        ..AtlasPreferences::default()
    };

    assert_eq!(
        service.update(invalid),
        Err(AtlasPreferencesError::InvalidPreference)
    );
    assert_eq!(service.status().unwrap(), AtlasPreferences::default());
}

#[test]
fn view_updates_are_ignored_until_the_user_opts_in() {
    let service = AtlasPreferencesService::new(MemoryAtlasPreferences::default());
    let view = AtlasView {
        longitude: 10.0,
        latitude: 20.0,
        zoom: 3.0,
        bearing: 0.0,
        pitch: 0.0,
    };

    assert_eq!(service.update_view(view).unwrap().last_view, None);
}

#[test]
fn camera_values_are_bounded_and_wrapped_before_storage() {
    let repository = MemoryAtlasPreferences::default();
    *repository.value.lock().unwrap() = Some(AtlasPreferences {
        restore_last_view: true,
        ..AtlasPreferences::default()
    });
    let service = AtlasPreferencesService::new(repository);

    let normalized = service
        .update_view(AtlasView {
            longitude: 541.0,
            latitude: 45.0,
            zoom: 8.0,
            bearing: 370.0,
            pitch: 40.0,
        })
        .unwrap()
        .last_view
        .unwrap();
    assert_eq!(normalized.longitude, -179.0);
    assert_eq!(normalized.bearing, 10.0);

    assert_eq!(
        service.update_view(AtlasView {
            latitude: 91.0,
            ..normalized
        }),
        Err(AtlasPreferencesError::InvalidPreference)
    );
    assert_eq!(
        service.update_view(AtlasView {
            zoom: f64::NAN,
            ..normalized
        }),
        Err(AtlasPreferencesError::InvalidPreference)
    );
}

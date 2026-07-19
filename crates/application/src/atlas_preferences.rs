//! Persisted Atlas behaviour and presentation choices.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wyrmgrid_storage::{AtlasPreferencesRecord, Store};

pub const AUTOMATIC_SYNC_MINUTE_OPTIONS: [u16; 5] = [0, 15, 30, 60, 120];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasLayerVisibility {
    pub daylight: bool,
    pub regions: bool,
    pub route: bool,
    pub fleet: bool,
    pub fbos: bool,
    pub airport_weather: bool,
    pub global_weather: bool,
    pub weather_coverage: bool,
    pub plugin_layers: bool,
}

impl Default for AtlasLayerVisibility {
    fn default() -> Self {
        Self {
            daylight: true,
            regions: true,
            route: true,
            fleet: true,
            fbos: true,
            airport_weather: true,
            global_weather: true,
            weather_coverage: true,
            plugin_layers: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtlasView {
    pub longitude: f64,
    pub latitude: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtlasPreferences {
    pub automatic_sync_minutes: u16,
    pub layers: AtlasLayerVisibility,
    pub restore_last_view: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_view: Option<AtlasView>,
}

impl Default for AtlasPreferences {
    fn default() -> Self {
        Self {
            automatic_sync_minutes: 30,
            layers: AtlasLayerVisibility::default(),
            restore_last_view: false,
            last_view: None,
        }
    }
}

pub trait AtlasPreferencesRepository: Send + Sync + 'static {
    fn load_atlas_preferences(&self) -> Result<Option<AtlasPreferences>, AtlasPreferencesError>;
    fn save_atlas_preferences(
        &self,
        preferences: AtlasPreferences,
    ) -> Result<(), AtlasPreferencesError>;
    fn save_atlas_view(&self, view: AtlasView) -> Result<(), AtlasPreferencesError>;
}

impl AtlasPreferencesRepository for Store {
    fn load_atlas_preferences(&self) -> Result<Option<AtlasPreferences>, AtlasPreferencesError> {
        self.load_atlas_preferences_record()
            .map_err(|_| AtlasPreferencesError::StorageUnavailable)?
            .map(record_to_preferences)
            .transpose()
    }

    fn save_atlas_preferences(
        &self,
        preferences: AtlasPreferences,
    ) -> Result<(), AtlasPreferencesError> {
        self.save_atlas_preferences_record(&preferences_to_record(preferences))
            .map_err(|_| AtlasPreferencesError::StorageUnavailable)
    }

    fn save_atlas_view(&self, view: AtlasView) -> Result<(), AtlasPreferencesError> {
        self.save_atlas_view_record(
            view.longitude,
            view.latitude,
            view.zoom,
            view.bearing,
            view.pitch,
        )
        .map_err(|_| AtlasPreferencesError::StorageUnavailable)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AtlasPreferencesError {
    #[error("WyrmGrid could not read or save its local Atlas settings.")]
    StorageUnavailable,
    #[error("That Atlas preference is outside the supported range.")]
    InvalidPreference,
}

pub struct AtlasPreferencesService<R> {
    repository: R,
}

impl<R: AtlasPreferencesRepository> AtlasPreferencesService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<AtlasPreferences, AtlasPreferencesError> {
        Ok(self
            .repository
            .load_atlas_preferences()?
            .unwrap_or_default())
    }

    pub fn update(
        &self,
        mut preferences: AtlasPreferences,
    ) -> Result<AtlasPreferences, AtlasPreferencesError> {
        if !AUTOMATIC_SYNC_MINUTE_OPTIONS.contains(&preferences.automatic_sync_minutes) {
            return Err(AtlasPreferencesError::InvalidPreference);
        }
        let current = self.status()?;
        preferences.last_view = preferences
            .restore_last_view
            .then_some(current.last_view)
            .flatten();
        self.repository.save_atlas_preferences(preferences)?;
        self.status()
    }

    pub fn update_view(&self, view: AtlasView) -> Result<AtlasPreferences, AtlasPreferencesError> {
        let preferences = self.status()?;
        if !preferences.restore_last_view {
            return Ok(preferences);
        }
        self.repository.save_atlas_view(validate_view(view)?)?;
        self.status()
    }
}

fn validate_view(mut view: AtlasView) -> Result<AtlasView, AtlasPreferencesError> {
    if !view.longitude.is_finite()
        || !view.latitude.is_finite()
        || !view.zoom.is_finite()
        || !view.bearing.is_finite()
        || !view.pitch.is_finite()
        || !(-90.0..=90.0).contains(&view.latitude)
        || !(0.0..=24.0).contains(&view.zoom)
        || !(0.0..=85.0).contains(&view.pitch)
    {
        return Err(AtlasPreferencesError::InvalidPreference);
    }
    view.longitude = normalize_angle(view.longitude);
    view.bearing = normalize_angle(view.bearing);
    Ok(view)
}

fn normalize_angle(value: f64) -> f64 {
    if (-180.0..=180.0).contains(&value) {
        value
    } else {
        (value + 180.0).rem_euclid(360.0) - 180.0
    }
}

fn preferences_to_record(preferences: AtlasPreferences) -> AtlasPreferencesRecord {
    AtlasPreferencesRecord {
        automatic_sync_minutes: preferences.automatic_sync_minutes,
        daylight_visible: preferences.layers.daylight,
        regions_visible: preferences.layers.regions,
        route_visible: preferences.layers.route,
        fleet_visible: preferences.layers.fleet,
        fbos_visible: preferences.layers.fbos,
        airport_weather_visible: preferences.layers.airport_weather,
        global_weather_visible: preferences.layers.global_weather,
        weather_coverage_visible: preferences.layers.weather_coverage,
        plugin_layers_visible: preferences.layers.plugin_layers,
        restore_last_view: preferences.restore_last_view,
        last_longitude: None,
        last_latitude: None,
        last_zoom: None,
        last_bearing: None,
        last_pitch: None,
    }
}

fn record_to_preferences(
    record: AtlasPreferencesRecord,
) -> Result<AtlasPreferences, AtlasPreferencesError> {
    if !AUTOMATIC_SYNC_MINUTE_OPTIONS.contains(&record.automatic_sync_minutes) {
        return Err(AtlasPreferencesError::InvalidPreference);
    }
    let camera_values = [
        record.last_longitude,
        record.last_latitude,
        record.last_zoom,
        record.last_bearing,
        record.last_pitch,
    ];
    let last_view = if camera_values.iter().all(Option::is_none) {
        None
    } else if let [
        Some(longitude),
        Some(latitude),
        Some(zoom),
        Some(bearing),
        Some(pitch),
    ] = camera_values
    {
        Some(validate_view(AtlasView {
            longitude,
            latitude,
            zoom,
            bearing,
            pitch,
        })?)
    } else {
        return Err(AtlasPreferencesError::InvalidPreference);
    };
    Ok(AtlasPreferences {
        automatic_sync_minutes: record.automatic_sync_minutes,
        layers: AtlasLayerVisibility {
            daylight: record.daylight_visible,
            regions: record.regions_visible,
            route: record.route_visible,
            fleet: record.fleet_visible,
            fbos: record.fbos_visible,
            airport_weather: record.airport_weather_visible,
            global_weather: record.global_weather_visible,
            weather_coverage: record.weather_coverage_visible,
            plugin_layers: record.plugin_layers_visible,
        },
        restore_last_view: record.restore_last_view,
        last_view: record.restore_last_view.then_some(last_view).flatten(),
    })
}

#[cfg(test)]
#[path = "tests/atlas_preferences.rs"]
mod tests;

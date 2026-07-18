//! Persisted presentation preferences. Raw facts remain in their canonical units.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wyrmgrid_storage::{DisplayPreferencesRecord, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltitudeUnit {
    Feet,
    Metres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedUnit {
    Knots,
    MilesPerHour,
    KilometresPerHour,
    MetresPerSecond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightUnit {
    Pounds,
    Kilograms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuelUnit {
    Pounds,
    Kilograms,
    UsGallons,
    ImperialGallons,
    Litres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherRenderingProfile {
    Compatibility,
    Enhanced,
    Cinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayPreferences {
    pub altitude_unit: AltitudeUnit,
    pub speed_unit: SpeedUnit,
    pub weight_unit: WeightUnit,
    pub fuel_unit: FuelUnit,
    pub responsive_surfaces: bool,
    pub weather_rendering_profile: WeatherRenderingProfile,
    pub weather_cloud_effects: bool,
    pub weather_precipitation_effects: bool,
    pub weather_lightning_effects: bool,
    pub weather_dust_effects: bool,
    pub reduce_weather_flashes: bool,
}

impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            altitude_unit: AltitudeUnit::Feet,
            speed_unit: SpeedUnit::Knots,
            weight_unit: WeightUnit::Pounds,
            fuel_unit: FuelUnit::Pounds,
            responsive_surfaces: true,
            weather_rendering_profile: WeatherRenderingProfile::Enhanced,
            weather_cloud_effects: true,
            weather_precipitation_effects: true,
            weather_lightning_effects: true,
            weather_dust_effects: true,
            reduce_weather_flashes: true,
        }
    }
}

pub trait DisplayPreferencesRepository: Send + Sync + 'static {
    fn load_display_preferences(&self) -> Result<Option<DisplayPreferences>, DisplaySettingsError>;
    fn save_display_preferences(
        &self,
        preferences: DisplayPreferences,
    ) -> Result<(), DisplaySettingsError>;
}

impl DisplayPreferencesRepository for Store {
    fn load_display_preferences(&self) -> Result<Option<DisplayPreferences>, DisplaySettingsError> {
        self.load_display_preferences_record()
            .map_err(|_| DisplaySettingsError::StorageUnavailable)?
            .map(record_to_preferences)
            .transpose()
    }

    fn save_display_preferences(
        &self,
        preferences: DisplayPreferences,
    ) -> Result<(), DisplaySettingsError> {
        self.save_display_preferences_record(&preferences_to_record(preferences))
            .map_err(|_| DisplaySettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DisplaySettingsError {
    #[error("WyrmGrid could not read or save its local display settings.")]
    StorageUnavailable,
    #[error("The saved display preferences are not supported by this WyrmGrid version.")]
    UnsupportedUnit,
}

pub struct DisplaySettingsService<R> {
    repository: R,
}

impl<R: DisplayPreferencesRepository> DisplaySettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<DisplayPreferences, DisplaySettingsError> {
        Ok(self
            .repository
            .load_display_preferences()?
            .unwrap_or_default())
    }

    pub fn update(
        &self,
        preferences: DisplayPreferences,
    ) -> Result<DisplayPreferences, DisplaySettingsError> {
        self.repository.save_display_preferences(preferences)?;
        self.status()
    }
}

fn preferences_to_record(preferences: DisplayPreferences) -> DisplayPreferencesRecord {
    DisplayPreferencesRecord {
        altitude_unit: match preferences.altitude_unit {
            AltitudeUnit::Feet => "feet",
            AltitudeUnit::Metres => "metres",
        }
        .to_owned(),
        speed_unit: match preferences.speed_unit {
            SpeedUnit::Knots => "knots",
            SpeedUnit::MilesPerHour => "miles_per_hour",
            SpeedUnit::KilometresPerHour => "kilometres_per_hour",
            SpeedUnit::MetresPerSecond => "metres_per_second",
        }
        .to_owned(),
        weight_unit: match preferences.weight_unit {
            WeightUnit::Pounds => "pounds",
            WeightUnit::Kilograms => "kilograms",
        }
        .to_owned(),
        fuel_unit: match preferences.fuel_unit {
            FuelUnit::Pounds => "pounds",
            FuelUnit::Kilograms => "kilograms",
            FuelUnit::UsGallons => "us_gallons",
            FuelUnit::ImperialGallons => "imperial_gallons",
            FuelUnit::Litres => "litres",
        }
        .to_owned(),
        responsive_surfaces: preferences.responsive_surfaces,
        weather_rendering_profile: match preferences.weather_rendering_profile {
            WeatherRenderingProfile::Compatibility => "compatibility",
            WeatherRenderingProfile::Enhanced => "enhanced",
            WeatherRenderingProfile::Cinematic => "cinematic",
        }
        .to_owned(),
        weather_cloud_effects: preferences.weather_cloud_effects,
        weather_precipitation_effects: preferences.weather_precipitation_effects,
        weather_lightning_effects: preferences.weather_lightning_effects,
        weather_dust_effects: preferences.weather_dust_effects,
        reduce_weather_flashes: preferences.reduce_weather_flashes,
    }
}

fn record_to_preferences(
    record: DisplayPreferencesRecord,
) -> Result<DisplayPreferences, DisplaySettingsError> {
    Ok(DisplayPreferences {
        altitude_unit: parse_unit(&record.altitude_unit)?,
        speed_unit: parse_unit(&record.speed_unit)?,
        weight_unit: parse_unit(&record.weight_unit)?,
        fuel_unit: parse_unit(&record.fuel_unit)?,
        responsive_surfaces: record.responsive_surfaces,
        weather_rendering_profile: parse_unit(&record.weather_rendering_profile)?,
        weather_cloud_effects: record.weather_cloud_effects,
        weather_precipitation_effects: record.weather_precipitation_effects,
        weather_lightning_effects: record.weather_lightning_effects,
        weather_dust_effects: record.weather_dust_effects,
        reduce_weather_flashes: record.reduce_weather_flashes,
    })
}

fn parse_unit<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T, DisplaySettingsError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| DisplaySettingsError::UnsupportedUnit)
}

#[cfg(test)]
#[path = "tests/display.rs"]
mod tests;

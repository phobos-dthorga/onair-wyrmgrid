//! Application-level orchestration independent of Tauri and other interfaces.

use chrono::{DateTime, SecondsFormat, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{AircraftSummary, CompanyId, CompanySummary, FboSummary, Observed};
use wyrmgrid_onair_api::{ClientError, DEFAULT_BASE_URL, OnAirClient};
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;
use wyrmgrid_storage::{ApiSnapshotRecord, Store};

const FLEET_RESOURCE_KIND: &str = "onair_company_fleet";
const FLEET_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const FBOS_RESOURCE_KIND: &str = "onair_company_fbos";
const FBOS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const MAX_HOARD_TIMELINE_OBSERVATIONS: usize = 4_096;
const UNKNOWN_AIRCRAFT_MODEL: &str = "Unknown model";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformStatus {
    pub application: &'static str,
    pub version: &'static str,
    pub plugin_api_version: u32,
    pub mode: &'static str,
}

pub fn platform_status() -> PlatformStatus {
    PlatformStatus {
        application: "OnAir WyrmGrid",
        version: env!("CARGO_PKG_VERSION"),
        plugin_api_version: PLUGIN_API_VERSION,
        mode: "foundation",
    }
}

pub const TERMS_VERSION: &str = "2026-07-14";
pub const PRIVACY_NOTICE_VERSION: &str = "2026-07-14";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalPreferences {
    pub terms_version: String,
    pub privacy_notice_version: String,
    pub telemetry_enabled: bool,
    pub acknowledged_at: String,
}

pub trait LegalPreferencesRepository: Send + Sync + 'static {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError>;

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError>;
}

impl LegalPreferencesRepository for Store {
    fn load_legal_preferences(
        &self,
    ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
        self.load_legal_preferences_record()
            .map(|preferences| {
                preferences.map(|preferences| PersistedLegalPreferences {
                    terms_version: preferences.terms_version,
                    privacy_notice_version: preferences.privacy_notice_version,
                    telemetry_enabled: preferences.telemetry_enabled,
                    acknowledged_at: preferences.acknowledged_at,
                })
            })
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }

    fn save_legal_preferences(
        &self,
        terms_version: &str,
        privacy_notice_version: &str,
        telemetry_enabled: bool,
    ) -> Result<(), LegalSettingsError> {
        self.save_legal_preferences_record(terms_version, privacy_notice_version, telemetry_enabled)
            .map_err(|_| LegalSettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegalStatus {
    pub terms_version: &'static str,
    pub privacy_notice_version: &'static str,
    pub acknowledged: bool,
    pub telemetry_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<String>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LegalSettingsError {
    #[error("WyrmGrid could not read or save its local privacy preferences.")]
    StorageUnavailable,
    #[error("Review the current Terms and Privacy Notice before changing this preference.")]
    AcknowledgementRequired,
}

pub struct LegalSettingsService<R> {
    repository: R,
}

impl<R: LegalPreferencesRepository> LegalSettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<LegalStatus, LegalSettingsError> {
        let stored = self.repository.load_legal_preferences()?;
        let acknowledged = stored.as_ref().is_some_and(|preferences| {
            preferences.terms_version == TERMS_VERSION
                && preferences.privacy_notice_version == PRIVACY_NOTICE_VERSION
        });
        let acknowledged_at = if acknowledged {
            stored
                .as_ref()
                .map(|preferences| preferences.acknowledged_at.clone())
        } else {
            None
        };

        Ok(LegalStatus {
            terms_version: TERMS_VERSION,
            privacy_notice_version: PRIVACY_NOTICE_VERSION,
            acknowledged,
            telemetry_enabled: acknowledged
                && stored
                    .as_ref()
                    .is_some_and(|preferences| preferences.telemetry_enabled),
            acknowledged_at,
        })
    }

    pub fn acknowledge(&self, telemetry_enabled: bool) -> Result<LegalStatus, LegalSettingsError> {
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }

    pub fn update_telemetry(
        &self,
        telemetry_enabled: bool,
    ) -> Result<LegalStatus, LegalSettingsError> {
        if !self.status()?.acknowledged {
            return Err(LegalSettingsError::AcknowledgementRequired);
        }
        self.repository.save_legal_preferences(
            TERMS_VERSION,
            PRIVACY_NOTICE_VERSION,
            telemetry_enabled,
        )?;
        self.status()
    }
}

pub const THEME_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "wyrmgrid-classic";
const MAX_THEME_MANIFEST_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCustomTheme {
    pub theme_id: String,
    pub manifest_json: String,
}

pub trait ThemeRepository: Send + Sync + 'static {
    fn load_selected_theme(&self) -> Result<Option<String>, ThemeSettingsError>;
    fn save_selected_theme(&self, theme_id: &str) -> Result<(), ThemeSettingsError>;
    fn list_custom_themes(&self) -> Result<Vec<PersistedCustomTheme>, ThemeSettingsError>;
    fn save_custom_theme(
        &self,
        theme_id: &str,
        manifest_json: &str,
    ) -> Result<(), ThemeSettingsError>;
}

impl ThemeRepository for Store {
    fn load_selected_theme(&self) -> Result<Option<String>, ThemeSettingsError> {
        self.load_theme_preferences_record()
            .map(|record| record.map(|record| record.selected_theme_id))
            .map_err(|_| ThemeSettingsError::StorageUnavailable)
    }

    fn save_selected_theme(&self, theme_id: &str) -> Result<(), ThemeSettingsError> {
        self.save_selected_theme_record(theme_id)
            .map_err(|_| ThemeSettingsError::StorageUnavailable)
    }

    fn list_custom_themes(&self) -> Result<Vec<PersistedCustomTheme>, ThemeSettingsError> {
        self.list_custom_theme_records()
            .map(|records| {
                records
                    .into_iter()
                    .map(|record| PersistedCustomTheme {
                        theme_id: record.theme_id,
                        manifest_json: record.manifest_json,
                    })
                    .collect()
            })
            .map_err(|_| ThemeSettingsError::StorageUnavailable)
    }

    fn save_custom_theme(
        &self,
        theme_id: &str,
        manifest_json: &str,
    ) -> Result<(), ThemeSettingsError> {
        self.save_custom_theme_record(theme_id, manifest_json)
            .map_err(|_| ThemeSettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub colors: ThemeColors,
    pub chart_palette: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeColors {
    pub canvas: String,
    pub surface: String,
    pub surface_elevated: String,
    pub surface_soft: String,
    pub text: String,
    pub text_muted: String,
    pub line: String,
    pub accent: String,
    pub highlight: String,
    pub danger: String,
    pub success: String,
    pub map_aircraft: String,
    pub map_fbo: String,
    pub map_label: String,
    pub map_halo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AvailableTheme {
    pub manifest: ThemeManifest,
    pub built_in: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeStatus {
    pub selected_theme_id: String,
    pub active_theme: ThemeManifest,
    pub themes: Vec<AvailableTheme>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSettingsError {
    #[error("WyrmGrid could not read or save its local theme settings.")]
    StorageUnavailable,
    #[error("That theme file is larger than the 32 KiB safety limit.")]
    ManifestTooLarge,
    #[error("That file is not a valid WyrmGrid theme manifest.")]
    InvalidManifest,
    #[error("That theme uses an unsupported manifest version.")]
    UnsupportedVersion,
    #[error("That theme identifier is invalid or reserved by WyrmGrid.")]
    InvalidIdentifier,
    #[error("That theme contains an invalid name or author.")]
    InvalidMetadata,
    #[error("Theme colors must use the #RRGGBB format.")]
    InvalidColour,
    #[error("That theme does not provide enough contrast for readable controls.")]
    InsufficientContrast,
    #[error("Choose a theme that is currently available.")]
    UnknownTheme,
}

pub struct ThemeSettingsService<R> {
    repository: R,
}

impl<R: ThemeRepository> ThemeSettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<ThemeStatus, ThemeSettingsError> {
        let mut themes = built_in_themes()
            .into_iter()
            .map(|manifest| AvailableTheme {
                manifest,
                built_in: true,
            })
            .collect::<Vec<_>>();

        for stored in self.repository.list_custom_themes()? {
            if let Ok(manifest) = parse_custom_theme(&stored.manifest_json)
                && manifest.id == stored.theme_id
            {
                themes.push(AvailableTheme {
                    manifest,
                    built_in: false,
                });
            }
        }

        let requested_id = self
            .repository
            .load_selected_theme()?
            .unwrap_or_else(|| DEFAULT_THEME_ID.to_owned());
        let active_theme = themes
            .iter()
            .find(|theme| theme.manifest.id == requested_id)
            .or_else(|| {
                themes
                    .iter()
                    .find(|theme| theme.manifest.id == DEFAULT_THEME_ID)
            })
            .expect("the default built-in theme must exist")
            .manifest
            .clone();

        Ok(ThemeStatus {
            selected_theme_id: active_theme.id.clone(),
            active_theme,
            themes,
        })
    }

    pub fn select(&self, theme_id: &str) -> Result<ThemeStatus, ThemeSettingsError> {
        let status = self.status()?;
        if !status
            .themes
            .iter()
            .any(|theme| theme.manifest.id == theme_id)
        {
            return Err(ThemeSettingsError::UnknownTheme);
        }
        self.repository.save_selected_theme(theme_id)?;
        self.status()
    }

    pub fn import(&self, manifest_json: &str) -> Result<ThemeStatus, ThemeSettingsError> {
        let manifest = parse_custom_theme(manifest_json)?;
        let canonical_json =
            serde_json::to_string(&manifest).map_err(|_| ThemeSettingsError::InvalidManifest)?;
        self.repository
            .save_custom_theme(&manifest.id, &canonical_json)?;
        self.repository.save_selected_theme(&manifest.id)?;
        self.status()
    }
}

pub fn built_in_themes() -> Vec<ThemeManifest> {
    vec![
        theme(
            "wyrmgrid-classic",
            "WyrmGrid Classic",
            ThemeColors {
                canvas: "#07110F".into(),
                surface: "#0A1916".into(),
                surface_elevated: "#102520".into(),
                surface_soft: "#172F29".into(),
                text: "#E9F1EF".into(),
                text_muted: "#A7B8B2".into(),
                line: "#526A62".into(),
                accent: "#73D6AD".into(),
                highlight: "#D5AE5F".into(),
                danger: "#ED8074".into(),
                success: "#73D6AD".into(),
                map_aircraft: "#D5AE5F".into(),
                map_fbo: "#73D6AD".into(),
                map_label: "#E9F1EF".into(),
                map_halo: "#07110F".into(),
            },
            ["#73D6AD", "#D5AE5F", "#72A7CF", "#CF7B73", "#A88BD4"],
        ),
        theme(
            "wyrmgrid-phobos",
            "Phobos D'thorga",
            ThemeColors {
                canvas: "#100708".into(),
                surface: "#1A0B0D".into(),
                surface_elevated: "#261014".into(),
                surface_soft: "#32161B".into(),
                text: "#F8EDEF".into(),
                text_muted: "#D7B8BE".into(),
                line: "#8C4A55".into(),
                accent: "#FF6B7A".into(),
                highlight: "#F2A65A".into(),
                danger: "#FF8A80".into(),
                success: "#8FE3B2".into(),
                map_aircraft: "#FF6B7A".into(),
                map_fbo: "#F2A65A".into(),
                map_label: "#F8EDEF".into(),
                map_halo: "#100708".into(),
            },
            ["#FF6B7A", "#F2A65A", "#E68AC3", "#A99CFF", "#8FE3B2"],
        ),
        theme(
            "wyrmgrid-daylight",
            "Daylight Dispatch",
            ThemeColors {
                canvas: "#F4F0E7".into(),
                surface: "#FBF9F3".into(),
                surface_elevated: "#FFFFFF".into(),
                surface_soft: "#E8E2D4".into(),
                text: "#14211E".into(),
                text_muted: "#52615C".into(),
                line: "#84928B".into(),
                accent: "#176B55".into(),
                highlight: "#7A500F".into(),
                danger: "#A13A32".into(),
                success: "#216E50".into(),
                map_aircraft: "#7A500F".into(),
                map_fbo: "#176B55".into(),
                map_label: "#14211E".into(),
                map_halo: "#FFFFFF".into(),
            },
            ["#176B55", "#7A500F", "#326A98", "#A13A32", "#6D4A91"],
        ),
        theme(
            "wyrmgrid-high-contrast",
            "High Contrast",
            ThemeColors {
                canvas: "#000000".into(),
                surface: "#0A0A0A".into(),
                surface_elevated: "#141414".into(),
                surface_soft: "#1E1E1E".into(),
                text: "#FFFFFF".into(),
                text_muted: "#D7D7D7".into(),
                line: "#858585".into(),
                accent: "#59FFBE".into(),
                highlight: "#FFD75A".into(),
                danger: "#FF786D".into(),
                success: "#6EFF9F".into(),
                map_aircraft: "#FFD75A".into(),
                map_fbo: "#59FFBE".into(),
                map_label: "#FFFFFF".into(),
                map_halo: "#000000".into(),
            },
            ["#59FFBE", "#FFD75A", "#67C8FF", "#FF786D", "#D39BFF"],
        ),
    ]
}

fn theme<const N: usize>(
    id: &str,
    name: &str,
    colors: ThemeColors,
    chart_palette: [&str; N],
) -> ThemeManifest {
    ThemeManifest {
        schema_version: THEME_SCHEMA_VERSION,
        id: id.into(),
        name: name.into(),
        author: Some("WyrmGrid".into()),
        colors,
        chart_palette: chart_palette.into_iter().map(str::to_owned).collect(),
    }
}

fn parse_custom_theme(manifest_json: &str) -> Result<ThemeManifest, ThemeSettingsError> {
    if manifest_json.len() > MAX_THEME_MANIFEST_BYTES {
        return Err(ThemeSettingsError::ManifestTooLarge);
    }
    let manifest: ThemeManifest =
        serde_json::from_str(manifest_json).map_err(|_| ThemeSettingsError::InvalidManifest)?;
    validate_theme(&manifest, false)?;
    Ok(manifest)
}

fn validate_theme(
    manifest: &ThemeManifest,
    allow_reserved_identifier: bool,
) -> Result<(), ThemeSettingsError> {
    if manifest.schema_version != THEME_SCHEMA_VERSION {
        return Err(ThemeSettingsError::UnsupportedVersion);
    }
    if !valid_theme_id(&manifest.id)
        || (!allow_reserved_identifier && manifest.id.starts_with("wyrmgrid-"))
    {
        return Err(ThemeSettingsError::InvalidIdentifier);
    }
    if !valid_label(&manifest.name, 64)
        || manifest
            .author
            .as_ref()
            .is_some_and(|author| !valid_label(author, 80))
    {
        return Err(ThemeSettingsError::InvalidMetadata);
    }

    let colors = [
        &manifest.colors.canvas,
        &manifest.colors.surface,
        &manifest.colors.surface_elevated,
        &manifest.colors.surface_soft,
        &manifest.colors.text,
        &manifest.colors.text_muted,
        &manifest.colors.line,
        &manifest.colors.accent,
        &manifest.colors.highlight,
        &manifest.colors.danger,
        &manifest.colors.success,
        &manifest.colors.map_aircraft,
        &manifest.colors.map_fbo,
        &manifest.colors.map_label,
        &manifest.colors.map_halo,
    ];
    if colors.into_iter().any(|colour| !valid_hex_colour(colour))
        || !(3..=8).contains(&manifest.chart_palette.len())
        || manifest
            .chart_palette
            .iter()
            .any(|colour| !valid_hex_colour(colour))
    {
        return Err(ThemeSettingsError::InvalidColour);
    }
    let interface_surfaces = [
        manifest.colors.canvas.as_str(),
        manifest.colors.surface.as_str(),
        manifest.colors.surface_elevated.as_str(),
        manifest.colors.surface_soft.as_str(),
    ];
    if !contrasts_with_all(&manifest.colors.text, &interface_surfaces, 4.5)
        || !contrasts_with_all(&manifest.colors.text_muted, &interface_surfaces, 4.5)
        || !contrasts_with_all(&manifest.colors.accent, &interface_surfaces, 4.5)
        || !contrasts_with_all(&manifest.colors.highlight, &interface_surfaces, 4.5)
        || !contrasts_with_all(&manifest.colors.danger, &interface_surfaces, 4.5)
        || !contrasts_with_all(&manifest.colors.success, &interface_surfaces, 4.5)
        || contrast_ratio(&manifest.colors.line, &manifest.colors.surface) < 1.5
        || contrast_ratio(&manifest.colors.map_label, &manifest.colors.map_halo) < 4.5
        || contrast_ratio(&manifest.colors.map_aircraft, &manifest.colors.map_halo) < 3.0
        || contrast_ratio(&manifest.colors.map_fbo, &manifest.colors.map_halo) < 3.0
        || contrast_ratio(&manifest.colors.highlight, &manifest.colors.map_halo) < 3.0
        || manifest
            .chart_palette
            .iter()
            .any(|colour| contrast_ratio(colour, &manifest.colors.surface) < 3.0)
    {
        return Err(ThemeSettingsError::InsufficientContrast);
    }
    Ok(())
}

fn valid_theme_id(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value.bytes().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
        })
        && !value.starts_with('-')
        && !value.ends_with('-')
}

fn valid_label(value: &str, maximum_length: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= maximum_length
        && !value.chars().any(char::is_control)
}

fn valid_hex_colour(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .bytes()
            .all(|character| character.is_ascii_hexdigit())
}

fn contrast_ratio(first: &str, second: &str) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn contrasts_with_all(foreground: &str, backgrounds: &[&str], minimum: f64) -> bool {
    backgrounds
        .iter()
        .all(|background| contrast_ratio(foreground, background) >= minimum)
}

fn relative_luminance(colour: &str) -> f64 {
    let channel = |offset| {
        let value = u8::from_str_radix(&colour[offset..offset + 2], 16)
            .expect("validated hex colours have complete channels") as f64
            / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub company: Option<ConnectedCompany>,
    pub credential_storage: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectedCompany {
    pub name: String,
    pub airline_code: String,
}

impl From<&CompanySummary> for ConnectedCompany {
    fn from(company: &CompanySummary) -> Self {
        Self {
            name: company.name.clone(),
            airline_code: company.airline_code.clone(),
        }
    }
}

pub const MANUAL_SYNC_COOLDOWN: Duration = Duration::from_secs(60);
pub const MINIMUM_AUTOMATIC_SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSyncTrigger {
    Initial,
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSyncDisposition {
    Synchronized,
    QuietlyIgnored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotAvailability {
    Live,
    Cached,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStorage {
    Hoard,
    MemoryOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<Vec<AircraftSummary>>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FboSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<Vec<FboSummary>>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyDataResource {
    Fleet,
    Fbos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataSyncFailure {
    pub resource: CompanyDataResource,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompanyDataSyncResult {
    pub disposition: DataSyncDisposition,
    pub fleet: Option<FleetSnapshotView>,
    pub fbos: Option<FboSnapshotView>,
    pub failures: Vec<DataSyncFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetHistoryPoint {
    pub observed_at: String,
    pub aircraft_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetCompositionPoint {
    pub model: String,
    pub aircraft_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HoardTimelineIndex {
    pub company: Option<ConnectedCompany>,
    pub observation_times: Vec<String>,
    pub fleet_history: Vec<FleetHistoryPoint>,
    pub current_fleet_composition: Vec<FleetCompositionPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoricalCompanyDataView {
    pub selected_at: String,
    pub fleet: Option<FleetSnapshotView>,
    pub fbos: Option<FboSnapshotView>,
    pub fleet_composition: Vec<FleetCompositionPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredFleetSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<Vec<AircraftSummary>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredFboSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<Vec<FboSummary>>,
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Enter a valid OnAir company ID.")]
    InvalidCompanyId,
    #[error("Enter your OnAir API key.")]
    EmptyApiKey,
    #[error(
        "OnAir rejected these details. For now, copy them from OnAir Client → Options → Global Settings—not OnAir Companion."
    )]
    AuthenticationRejected,
    #[error("That company was not found in the selected OnAir world.")]
    CompanyNotFound,
    #[error("OnAir is receiving too many requests. Please wait before trying again.")]
    RateLimited,
    #[error("WyrmGrid could not reach OnAir. Check your connection and try again.")]
    ServiceUnavailable,
    #[error("The local connection state is unavailable.")]
    StateUnavailable,
    #[error("Connect to OnAir before synchronizing company data.")]
    NotConnected,
    #[error(
        "WyrmGrid could not refresh the fleet. A previous successful observation, if present, remains available."
    )]
    FleetUnavailable,
    #[error(
        "WyrmGrid could not refresh the FBO network. A previous successful observation, if present, remains available."
    )]
    FbosUnavailable,
}

#[derive(Debug, Error)]
pub enum HoardTimelineError {
    #[error("The selected Hoard time is invalid.")]
    InvalidSelection,
    #[error("No retained company observation exists at that time.")]
    ObservationUnavailable,
    #[error("WyrmGrid could not read the local Hoard timeline.")]
    StorageUnavailable,
    #[error("The local timeline state is unavailable.")]
    StateUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub reportable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_id: Option<String>,
}

impl OperationError {
    pub fn with_report_id(mut self, report_id: Option<String>) -> Self {
        self.report_id = report_id;
        self
    }
}

impl From<ConnectionError> for OperationError {
    fn from(error: ConnectionError) -> Self {
        let (code, retryable, reportable) = match &error {
            ConnectionError::InvalidCompanyId => ("onair.invalid_company_id", false, false),
            ConnectionError::EmptyApiKey => ("onair.empty_api_key", false, false),
            ConnectionError::AuthenticationRejected => {
                ("onair.authentication_rejected", false, false)
            }
            ConnectionError::CompanyNotFound => ("onair.company_not_found", false, false),
            ConnectionError::RateLimited => ("onair.rate_limited", true, false),
            ConnectionError::ServiceUnavailable => ("onair.service_unavailable", true, false),
            ConnectionError::StateUnavailable => ("application.state_unavailable", true, true),
            ConnectionError::NotConnected => ("onair.not_connected", false, false),
            ConnectionError::FleetUnavailable => ("onair.fleet_unavailable", true, false),
            ConnectionError::FbosUnavailable => ("onair.fbos_unavailable", true, false),
        };

        Self {
            code,
            message: error.to_string(),
            retryable,
            reportable,
            report_id: None,
        }
    }
}

impl From<HoardTimelineError> for OperationError {
    fn from(error: HoardTimelineError) -> Self {
        let (code, retryable, reportable) = match error {
            HoardTimelineError::InvalidSelection => {
                ("hoard.invalid_timeline_selection", false, false)
            }
            HoardTimelineError::ObservationUnavailable => {
                ("hoard.observation_unavailable", false, false)
            }
            HoardTimelineError::StorageUnavailable => ("hoard.storage_unavailable", true, true),
            HoardTimelineError::StateUnavailable => ("application.state_unavailable", true, true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
            reportable,
            report_id: None,
        }
    }
}

impl From<LegalSettingsError> for OperationError {
    fn from(error: LegalSettingsError) -> Self {
        let code = match error {
            LegalSettingsError::StorageUnavailable => "legal.storage_unavailable",
            LegalSettingsError::AcknowledgementRequired => "legal.acknowledgement_required",
        };
        Self {
            code,
            message: error.to_string(),
            retryable: matches!(error, LegalSettingsError::StorageUnavailable),
            reportable: false,
            report_id: None,
        }
    }
}

impl From<ThemeSettingsError> for OperationError {
    fn from(error: ThemeSettingsError) -> Self {
        let (code, retryable) = match error {
            ThemeSettingsError::StorageUnavailable => ("theme.storage_unavailable", true),
            ThemeSettingsError::ManifestTooLarge => ("theme.manifest_too_large", false),
            ThemeSettingsError::InvalidManifest => ("theme.invalid_manifest", false),
            ThemeSettingsError::UnsupportedVersion => ("theme.unsupported_version", false),
            ThemeSettingsError::InvalidIdentifier => ("theme.invalid_identifier", false),
            ThemeSettingsError::InvalidMetadata => ("theme.invalid_metadata", false),
            ThemeSettingsError::InvalidColour => ("theme.invalid_colour", false),
            ThemeSettingsError::InsufficientContrast => ("theme.insufficient_contrast", false),
            ThemeSettingsError::UnknownTheme => ("theme.unknown_theme", false),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
            reportable: false,
            report_id: None,
        }
    }
}

#[derive(Clone)]
pub struct OnAirSession {
    inner: Arc<RwLock<Option<ConnectedSession>>>,
    fleet: Arc<RwLock<Option<FleetSnapshotView>>>,
    fbos: Arc<RwLock<Option<FboSnapshotView>>>,
    store: Arc<Mutex<Store>>,
    base_url: &'static str,
}

struct ConnectedSession {
    client: Arc<OnAirClient>,
    company: CompanySummary,
    data_sync_gate: Arc<Mutex<DataSyncGate>>,
}

#[derive(Debug, Default)]
struct DataSyncGate {
    in_progress: bool,
    last_started: Option<Instant>,
}

impl DataSyncGate {
    fn try_start(&mut self, trigger: DataSyncTrigger, now: Instant) -> bool {
        if self.in_progress {
            return false;
        }

        let minimum_interval = match trigger {
            DataSyncTrigger::Initial => Duration::ZERO,
            DataSyncTrigger::Manual => MANUAL_SYNC_COOLDOWN,
            DataSyncTrigger::Automatic => MINIMUM_AUTOMATIC_SYNC_INTERVAL,
        };
        if self
            .last_started
            .is_some_and(|last_started| now.duration_since(last_started) < minimum_interval)
        {
            return false;
        }

        self.in_progress = true;
        self.last_started = Some(now);
        true
    }

    fn finish(&mut self) {
        self.in_progress = false;
    }
}

struct DataSyncPermit {
    gate: Arc<Mutex<DataSyncGate>>,
}

impl Drop for DataSyncPermit {
    fn drop(&mut self) {
        if let Ok(mut gate) = self.gate.lock() {
            gate.finish();
        }
    }
}

impl Default for OnAirSession {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL)
    }
}

impl OnAirSession {
    pub fn new(base_url: &'static str) -> Self {
        let store = Store::open_in_memory().expect("in-memory Hoard should initialize");
        Self::with_store(base_url, store)
    }

    pub fn with_store(base_url: &'static str, store: Store) -> Self {
        let persistent = store.is_persistent();
        let stored_fleet = load_stored_fleet(&store, None);
        let anchor_company = stored_fleet
            .as_ref()
            .map(|stored| stored.company.id.clone());
        let stored_fbos = load_stored_fbos(&store, anchor_company.as_ref());
        let storage = if persistent {
            SnapshotStorage::Hoard
        } else {
            SnapshotStorage::MemoryOnly
        };
        let cached_fleet =
            stored_fleet.map(|stored| fleet_view(stored, SnapshotAvailability::Offline, storage));
        let cached_fbos =
            stored_fbos.map(|stored| fbo_view(stored, SnapshotAvailability::Offline, storage));
        Self {
            inner: Arc::new(RwLock::new(None)),
            fleet: Arc::new(RwLock::new(cached_fleet)),
            fbos: Arc::new(RwLock::new(cached_fbos)),
            store: Arc::new(Mutex::new(store)),
            base_url,
        }
    }

    pub fn with_default_store(store: Store) -> Self {
        Self::with_store(DEFAULT_BASE_URL, store)
    }

    pub async fn connect(
        &self,
        company_id: String,
        api_key: String,
    ) -> Result<ConnectionStatus, ConnectionError> {
        let company_id =
            Uuid::parse_str(company_id.trim()).map_err(|_| ConnectionError::InvalidCompanyId)?;
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(ConnectionError::EmptyApiKey);
        }

        let client = Arc::new(
            OnAirClient::new(
                self.base_url,
                company_id,
                SecretString::from(api_key.to_owned()),
            )
            .map_err(classify_client_error)?,
        );
        let company = client
            .company_summary()
            .await
            .map_err(classify_client_error)?;

        let (cached_fleet, cached_fbos) = self.store.lock().ok().map_or((None, None), |store| {
            let storage = if store.is_persistent() {
                SnapshotStorage::Hoard
            } else {
                SnapshotStorage::MemoryOnly
            };
            (
                load_stored_fleet(&store, Some(&company.id))
                    .map(|stored| fleet_view(stored, SnapshotAvailability::Cached, storage)),
                load_stored_fbos(&store, Some(&company.id))
                    .map(|stored| fbo_view(stored, SnapshotAvailability::Cached, storage)),
            )
        });

        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(ConnectedSession {
            client,
            company,
            data_sync_gate: Arc::new(Mutex::new(DataSyncGate::default())),
        });
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_fleet;
        *self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_fbos;

        self.status()
    }

    pub fn disconnect(&self) -> Result<ConnectionStatus, ConnectionError> {
        *self
            .inner
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = None;
        if let Some(fleet) = self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            fleet.availability = SnapshotAvailability::Offline;
        }
        if let Some(fbos) = self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            fbos.availability = SnapshotAvailability::Offline;
        }
        self.status()
    }

    pub fn status(&self) -> Result<ConnectionStatus, ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        Ok(ConnectionStatus {
            connected: session.is_some(),
            company: session
                .as_ref()
                .map(|connected| ConnectedCompany::from(&connected.company)),
            credential_storage: "session_only",
        })
    }

    pub async fn synchronize_company_data(
        &self,
        trigger: DataSyncTrigger,
    ) -> Result<CompanyDataSyncResult, ConnectionError> {
        let (company, client, data_sync_gate) = {
            let session = self
                .inner
                .read()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
            (
                connected.company.clone(),
                Arc::clone(&connected.client),
                Arc::clone(&connected.data_sync_gate),
            )
        };

        let _sync_permit = {
            let mut gate = data_sync_gate
                .lock()
                .map_err(|_| ConnectionError::StateUnavailable)?;
            if !gate.try_start(trigger, Instant::now()) {
                return Ok(CompanyDataSyncResult {
                    disposition: DataSyncDisposition::QuietlyIgnored,
                    fleet: self.fleet_snapshot()?,
                    fbos: self.fbo_snapshot()?,
                    failures: Vec::new(),
                });
            }
            DataSyncPermit {
                gate: Arc::clone(&data_sync_gate),
            }
        };

        let mut failures = Vec::new();
        let mut stop_after_fleet = false;
        let fleet = match client.fleet().await {
            Ok(snapshot) => Some(self.accept_fleet_snapshot(&company, snapshot)?),
            Err(error) => {
                stop_after_fleet = matches!(
                    error,
                    ClientError::AuthenticationRejected | ClientError::RateLimited
                );
                self.mark_fleet_cached(&company.id)?;
                failures.push(DataSyncFailure {
                    resource: CompanyDataResource::Fleet,
                    message: classify_resource_error(error, CompanyDataResource::Fleet).to_string(),
                });
                self.fleet_snapshot()?
            }
        };

        let fbos = if stop_after_fleet {
            self.mark_fbos_cached(&company.id)?;
            failures.push(DataSyncFailure {
                resource: CompanyDataResource::Fbos,
                message: "FBO synchronization was skipped to avoid another rejected request."
                    .to_owned(),
            });
            self.fbo_snapshot()?
        } else {
            match client.fbos().await {
                Ok(snapshot) => Some(self.accept_fbo_snapshot(&company, snapshot)?),
                Err(error) => {
                    self.mark_fbos_cached(&company.id)?;
                    failures.push(DataSyncFailure {
                        resource: CompanyDataResource::Fbos,
                        message: classify_resource_error(error, CompanyDataResource::Fbos)
                            .to_string(),
                    });
                    self.fbo_snapshot()?
                }
            }
        };

        Ok(CompanyDataSyncResult {
            disposition: DataSyncDisposition::Synchronized,
            fleet,
            fbos,
            failures,
        })
    }

    fn accept_fleet_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<Vec<AircraftSummary>>,
    ) -> Result<FleetSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        let stored = StoredFleetSnapshot {
            schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_fleet(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = fleet_view(stored, SnapshotAvailability::Live, storage);
        *self
            .fleet
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(view)
    }

    fn accept_fbo_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<Vec<FboSummary>>,
    ) -> Result<FboSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        let stored = StoredFboSnapshot {
            schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_fbos(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = fbo_view(stored, SnapshotAvailability::Live, storage);
        *self
            .fbos
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(view)
    }

    pub fn fleet_snapshot(&self) -> Result<Option<FleetSnapshotView>, ConnectionError> {
        self.fleet
            .read()
            .map(|fleet| fleet.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    pub fn fbo_snapshot(&self) -> Result<Option<FboSnapshotView>, ConnectionError> {
        self.fbos
            .read()
            .map(|fbos| fbos.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    pub fn hoard_timeline_index(&self) -> Result<HoardTimelineIndex, HoardTimelineError> {
        let connected_company = self
            .inner
            .read()
            .map_err(|_| HoardTimelineError::StateUnavailable)?
            .as_ref()
            .map(|session| session.company.clone());
        let current_fleet = self
            .fleet
            .read()
            .map_err(|_| HoardTimelineError::StateUnavailable)?
            .clone();
        let store = self
            .store
            .lock()
            .map_err(|_| HoardTimelineError::StateUnavailable)?;
        let company = connected_company
            .or_else(|| load_stored_fleet(&store, None).map(|stored| stored.company))
            .or_else(|| load_stored_fbos(&store, None).map(|stored| stored.company));
        let Some(company) = company else {
            return Ok(HoardTimelineIndex {
                company: None,
                observation_times: Vec::new(),
                fleet_history: Vec::new(),
                current_fleet_composition: Vec::new(),
            });
        };

        let resource_key = company.id.0.to_string();
        let fleet_records = store
            .api_snapshot_history(
                FLEET_RESOURCE_KIND,
                &resource_key,
                MAX_HOARD_TIMELINE_OBSERVATIONS,
            )
            .map_err(|_| HoardTimelineError::StorageUnavailable)?;
        let fbo_records = store
            .api_snapshot_history(
                FBOS_RESOURCE_KIND,
                &resource_key,
                MAX_HOARD_TIMELINE_OBSERVATIONS,
            )
            .map_err(|_| HoardTimelineError::StorageUnavailable)?;

        let valid_fleet = fleet_records
            .into_iter()
            .filter_map(stored_fleet_from_record)
            .collect::<Vec<_>>();
        let valid_fbos = fbo_records
            .into_iter()
            .filter_map(stored_fbos_from_record)
            .collect::<Vec<_>>();
        let observation_times = valid_fleet
            .iter()
            .map(|stored| stored.snapshot.provenance.observed_at)
            .chain(
                valid_fbos
                    .iter()
                    .map(|stored| stored.snapshot.provenance.observed_at),
            )
            .map(format_timeline_time)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let fleet_history = valid_fleet
            .iter()
            .map(|stored| FleetHistoryPoint {
                observed_at: format_timeline_time(stored.snapshot.provenance.observed_at),
                aircraft_count: bounded_count(stored.snapshot.value.len()),
            })
            .collect();
        let current_fleet_composition = current_fleet
            .as_ref()
            .map(|view| fleet_composition(&view.snapshot.value))
            .unwrap_or_default();

        Ok(HoardTimelineIndex {
            company: Some(ConnectedCompany::from(&company)),
            observation_times,
            fleet_history,
            current_fleet_composition,
        })
    }

    pub fn historical_company_data(
        &self,
        selected_at: &str,
    ) -> Result<HistoricalCompanyDataView, HoardTimelineError> {
        if selected_at.len() > 64 {
            return Err(HoardTimelineError::InvalidSelection);
        }
        let selected_at = DateTime::parse_from_rfc3339(selected_at)
            .map_err(|_| HoardTimelineError::InvalidSelection)?
            .with_timezone(&Utc);
        let selected_at_text = format_timeline_time(selected_at);
        let connected_company = self
            .inner
            .read()
            .map_err(|_| HoardTimelineError::StateUnavailable)?
            .as_ref()
            .map(|session| session.company.clone());
        let store = self
            .store
            .lock()
            .map_err(|_| HoardTimelineError::StateUnavailable)?;
        let company = connected_company
            .clone()
            .or_else(|| load_stored_fleet(&store, None).map(|stored| stored.company))
            .or_else(|| load_stored_fbos(&store, None).map(|stored| stored.company))
            .ok_or(HoardTimelineError::ObservationUnavailable)?;
        let resource_key = company.id.0.to_string();
        let availability = if connected_company
            .as_ref()
            .is_some_and(|connected| connected.id == company.id)
        {
            SnapshotAvailability::Cached
        } else {
            SnapshotAvailability::Offline
        };
        let storage = if store.is_persistent() {
            SnapshotStorage::Hoard
        } else {
            SnapshotStorage::MemoryOnly
        };
        let fleet = store
            .api_snapshot_history_at_or_before(
                FLEET_RESOURCE_KIND,
                &resource_key,
                &selected_at_text,
                MAX_HOARD_TIMELINE_OBSERVATIONS,
            )
            .map_err(|_| HoardTimelineError::StorageUnavailable)?
            .into_iter()
            .rev()
            .find_map(stored_fleet_from_record)
            .map(|stored| fleet_view(stored, availability, storage));
        let fbos = store
            .api_snapshot_history_at_or_before(
                FBOS_RESOURCE_KIND,
                &resource_key,
                &selected_at_text,
                MAX_HOARD_TIMELINE_OBSERVATIONS,
            )
            .map_err(|_| HoardTimelineError::StorageUnavailable)?
            .into_iter()
            .rev()
            .find_map(stored_fbos_from_record)
            .map(|stored| fbo_view(stored, availability, storage));
        if fleet.is_none() && fbos.is_none() {
            return Err(HoardTimelineError::ObservationUnavailable);
        }
        let fleet_composition = fleet
            .as_ref()
            .map(|view| fleet_composition(&view.snapshot.value))
            .unwrap_or_default();

        Ok(HistoricalCompanyDataView {
            selected_at: selected_at_text,
            fleet,
            fbos,
            fleet_composition,
        })
    }

    fn ensure_current_company(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let session = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        let connected = session.as_ref().ok_or(ConnectionError::NotConnected)?;
        (&connected.company.id == company_id)
            .then_some(())
            .ok_or(ConnectionError::StateUnavailable)
    }

    fn mark_fleet_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(fleet) = self
                .fleet
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            fleet.availability = SnapshotAvailability::Cached;
        }
        Ok(())
    }

    fn mark_fbos_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(fbos) = self
                .fbos
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            fbos.availability = SnapshotAvailability::Cached;
        }
        Ok(())
    }
}

fn fleet_view(
    stored: StoredFleetSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> FleetSnapshotView {
    FleetSnapshotView {
        company: ConnectedCompany::from(&stored.company),
        snapshot: stored.snapshot,
        availability,
        storage,
    }
}

fn fbo_view(
    stored: StoredFboSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> FboSnapshotView {
    FboSnapshotView {
        company: ConnectedCompany::from(&stored.company),
        snapshot: stored.snapshot,
        availability,
        storage,
    }
}

fn load_stored_fleet(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredFleetSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(FLEET_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    stored_fleet_from_record(record)
}

fn stored_fleet_from_record(record: ApiSnapshotRecord) -> Option<StoredFleetSnapshot> {
    let stored: StoredFleetSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == FLEET_SNAPSHOT_SCHEMA_VERSION
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_fleet(store: &mut Store, stored: &StoredFleetSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            FLEET_RESOURCE_KIND,
            &stored.company.id.0.to_string(),
            &stored.snapshot.provenance.observed_at.to_rfc3339(),
            &payload,
        )
        .map_err(|_| ())
}

fn load_stored_fbos(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredFboSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(FBOS_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    stored_fbos_from_record(record)
}

fn stored_fbos_from_record(record: ApiSnapshotRecord) -> Option<StoredFboSnapshot> {
    let stored: StoredFboSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == FBOS_SNAPSHOT_SCHEMA_VERSION
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_fbos(store: &mut Store, stored: &StoredFboSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            FBOS_RESOURCE_KIND,
            &stored.company.id.0.to_string(),
            &stored.snapshot.provenance.observed_at.to_rfc3339(),
            &payload,
        )
        .map_err(|_| ())
}

fn format_timeline_time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn bounded_count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn fleet_composition(aircraft: &[AircraftSummary]) -> Vec<FleetCompositionPoint> {
    let mut counts = BTreeMap::<String, u32>::new();
    for item in aircraft {
        let model = item
            .model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .unwrap_or(UNKNOWN_AIRCRAFT_MODEL);
        let count = counts.entry(model.to_owned()).or_default();
        *count = count.saturating_add(1);
    }
    let mut composition = counts
        .into_iter()
        .map(|(model, aircraft_count)| FleetCompositionPoint {
            model,
            aircraft_count,
        })
        .collect::<Vec<_>>();
    composition.sort_by(|left, right| {
        right
            .aircraft_count
            .cmp(&left.aircraft_count)
            .then_with(|| left.model.cmp(&right.model))
    });
    composition
}

fn classify_client_error(error: ClientError) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected | ClientError::ApiRejected => {
            ConnectionError::AuthenticationRejected
        }
        ClientError::CompanyNotFound => ConnectionError::CompanyNotFound,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => ConnectionError::ServiceUnavailable,
    }
}

fn classify_resource_error(error: ClientError, resource: CompanyDataResource) -> ConnectionError {
    match error {
        ClientError::AuthenticationRejected => ConnectionError::AuthenticationRejected,
        ClientError::RateLimited => ConnectionError::RateLimited,
        _ => match resource {
            CompanyDataResource::Fleet => ConnectionError::FleetUnavailable,
            CompanyDataResource::Fbos => ConnectionError::FbosUnavailable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Timelike, Utc};
    use tempfile::tempdir;
    use wyrmgrid_domain::{
        AircraftId, AirportId, AirportSummary, FboId, Provenance, ProvenanceKind,
    };

    #[derive(Default)]
    struct MemoryLegalPreferences {
        value: Mutex<Option<PersistedLegalPreferences>>,
    }

    impl LegalPreferencesRepository for MemoryLegalPreferences {
        fn load_legal_preferences(
            &self,
        ) -> Result<Option<PersistedLegalPreferences>, LegalSettingsError> {
            self.value
                .lock()
                .map(|value| value.clone())
                .map_err(|_| LegalSettingsError::StorageUnavailable)
        }

        fn save_legal_preferences(
            &self,
            terms_version: &str,
            privacy_notice_version: &str,
            telemetry_enabled: bool,
        ) -> Result<(), LegalSettingsError> {
            *self
                .value
                .lock()
                .map_err(|_| LegalSettingsError::StorageUnavailable)? =
                Some(PersistedLegalPreferences {
                    terms_version: terms_version.to_owned(),
                    privacy_notice_version: privacy_notice_version.to_owned(),
                    telemetry_enabled,
                    acknowledged_at: "2026-07-14 00:00:00".to_owned(),
                });
            Ok(())
        }
    }

    #[test]
    fn exposes_the_supported_plugin_api() {
        assert_eq!(platform_status().plugin_api_version, 1);
    }

    #[test]
    fn built_in_themes_satisfy_the_same_safety_rules() {
        let themes = built_in_themes();
        assert!(
            themes
                .iter()
                .any(|theme| { theme.id == "wyrmgrid-phobos" && theme.name == "Phobos D'thorga" })
        );
        for theme in themes {
            validate_theme(&theme, true).expect("built-in theme should remain valid");
        }
    }

    #[test]
    fn imports_and_selects_a_data_only_custom_theme() {
        let service = ThemeSettingsService::new(
            Store::open_in_memory().expect("theme store should initialize"),
        );
        let manifest = include_str!("../../../schemas/fixtures/theme-manifest-v1.json");

        let status = service.import(manifest).expect("valid theme should import");
        assert_eq!(status.selected_theme_id, "midnight-cargo");
        assert_eq!(status.active_theme.name, "Midnight Cargo");
        assert_eq!(status.themes.len(), 5);

        let selected = service
            .select(DEFAULT_THEME_ID)
            .expect("built-in theme should be selectable");
        assert_eq!(selected.selected_theme_id, DEFAULT_THEME_ID);
    }

    #[test]
    fn rejects_theme_code_reserved_identifiers_and_low_contrast() {
        let arbitrary_css = r##"{
            "schema_version":1,"id":"custom","name":"Unsafe","css":"body{}",
            "colors":{},"chart_palette":["#FFFFFF","#000000","#777777"]
        }"##;
        assert_eq!(
            parse_custom_theme(arbitrary_css),
            Err(ThemeSettingsError::InvalidManifest)
        );

        let mut reserved = built_in_themes().remove(0);
        assert_eq!(
            validate_theme(&reserved, false),
            Err(ThemeSettingsError::InvalidIdentifier)
        );
        reserved.id = "low-contrast".into();
        reserved.colors.text = reserved.colors.canvas.clone();
        assert_eq!(
            validate_theme(&reserved, false),
            Err(ThemeSettingsError::InsufficientContrast)
        );
    }

    #[test]
    fn corrupt_or_missing_selected_themes_fall_back_safely() {
        let store = Store::open_in_memory().expect("theme store should initialize");
        store
            .save_custom_theme_record("broken-theme", "{not-json}")
            .expect("corrupt fixture should save at the raw storage boundary");
        store
            .save_selected_theme_record("broken-theme")
            .expect("selected fixture should save");
        let service = ThemeSettingsService::new(store);

        let status = service
            .status()
            .expect("theme status should degrade safely");
        assert_eq!(status.selected_theme_id, DEFAULT_THEME_ID);
        assert_eq!(status.active_theme.id, DEFAULT_THEME_ID);
        assert_eq!(status.themes.len(), 4);
    }

    #[test]
    fn legal_documents_require_versioned_acknowledgement() {
        let service = LegalSettingsService::new(MemoryLegalPreferences::default());
        assert_eq!(
            service.status().expect("status should be available"),
            LegalStatus {
                terms_version: TERMS_VERSION,
                privacy_notice_version: PRIVACY_NOTICE_VERSION,
                acknowledged: false,
                telemetry_enabled: false,
                acknowledged_at: None,
            }
        );

        let accepted = service
            .acknowledge(true)
            .expect("preferences should be saved");
        assert!(accepted.acknowledged);
        assert!(accepted.telemetry_enabled);
        assert_eq!(
            accepted.acknowledged_at.as_deref(),
            Some("2026-07-14 00:00:00")
        );

        let updated = service
            .update_telemetry(false)
            .expect("telemetry preference should be saved");
        assert!(!updated.telemetry_enabled);
    }

    #[test]
    fn old_legal_versions_disable_telemetry_until_reviewed() {
        let repository = MemoryLegalPreferences::default();
        repository
            .save_legal_preferences("2026-01-01", "2026-01-01", true)
            .expect("fixture should be saved");
        let service = LegalSettingsService::new(repository);

        let status = service.status().expect("status should be available");
        assert!(!status.acknowledged);
        assert!(!status.telemetry_enabled);
        assert!(matches!(
            service.update_telemetry(true),
            Err(LegalSettingsError::AcknowledgementRequired)
        ));
    }

    #[test]
    fn starts_disconnected_without_persistent_credentials() {
        let session = OnAirSession::default();
        assert_eq!(
            session.status().expect("status should be available"),
            ConnectionStatus {
                connected: false,
                company: None,
                credential_storage: "session_only",
            }
        );
    }

    #[test]
    fn restores_the_latest_persistent_company_data_as_offline() {
        let directory = tempdir().expect("temporary Hoard directory should exist");
        let database_path = directory.path().join("wyrmgrid.db");
        let company = CompanySummary {
            id: CompanyId(Uuid::new_v4()),
            name: "Cached Charter".into(),
            airline_code: "CCH".into(),
        };
        let stored = StoredFleetSnapshot {
            schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot: Observed {
                value: vec![AircraftSummary {
                    id: AircraftId(Uuid::new_v4()),
                    registration: Some("CACHE-1".into()),
                    model: Some("Stored Aircraft".into()),
                    location: None,
                    current_airport: None,
                }],
                provenance: Provenance {
                    kind: ProvenanceKind::OnAirFact,
                    source: "onair:company/fleet".into(),
                    observed_at: Utc::now(),
                },
            },
        };
        let stored_fbos = StoredFboSnapshot {
            schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot: Observed {
                value: vec![FboSummary {
                    id: FboId(Uuid::new_v4()),
                    name: Some("Cached Aerie".into()),
                    airport: Some(AirportSummary {
                        id: AirportId(Uuid::new_v4()),
                        icao: Some("YTEST".into()),
                        name: Some("Stored Airport".into()),
                        location: None,
                    }),
                }],
                provenance: Provenance {
                    kind: ProvenanceKind::OnAirFact,
                    source: "onair:company/fbos".into(),
                    observed_at: Utc::now(),
                },
            },
        };
        let mut store = Store::open(&database_path).expect("persistent Hoard should open");
        save_stored_fleet(&mut store, &stored).expect("fleet should persist");
        save_stored_fbos(&mut store, &stored_fbos).expect("FBOs should persist");
        drop(store);

        let session = OnAirSession::with_store(
            DEFAULT_BASE_URL,
            Store::open(&database_path).expect("persistent Hoard should reopen"),
        );
        let fleet_view = session
            .fleet_snapshot()
            .expect("fleet state should be readable")
            .expect("cached fleet should restore");
        let fbo_view = session
            .fbo_snapshot()
            .expect("FBO state should be readable")
            .expect("cached FBOs should restore");

        assert_eq!(fleet_view.company, ConnectedCompany::from(&company));
        assert_eq!(fleet_view.availability, SnapshotAvailability::Offline);
        assert_eq!(fleet_view.storage, SnapshotStorage::Hoard);
        assert_eq!(fleet_view.snapshot, stored.snapshot);
        assert_eq!(fbo_view.company, ConnectedCompany::from(&company));
        assert_eq!(fbo_view.availability, SnapshotAvailability::Offline);
        assert_eq!(fbo_view.storage, SnapshotStorage::Hoard);
        assert_eq!(fbo_view.snapshot, stored_fbos.snapshot);
    }

    #[test]
    fn builds_a_timeline_and_resolves_company_data_as_of_a_retained_time() {
        let company = CompanySummary {
            id: CompanyId(Uuid::new_v4()),
            name: "Timeline Charter".into(),
            airline_code: "TLC".into(),
        };
        let latest_hour = Utc::now()
            .with_minute(0)
            .and_then(|value| value.with_second(0))
            .and_then(|value| value.with_nanosecond(0))
            .expect("current hour should be representable");
        let mut store = Store::open_in_memory().expect("timeline store should initialize");
        for (offset, models) in [
            (-2, vec!["Cessna 172"]),
            (-1, vec!["Cessna 172", "Beechcraft King Air"]),
            (0, vec!["Cessna 172", "Beechcraft King Air", "Cessna 172"]),
        ] {
            let observed_at = latest_hour + ChronoDuration::hours(offset);
            let aircraft = models
                .into_iter()
                .map(|model| AircraftSummary {
                    id: AircraftId(Uuid::new_v4()),
                    registration: None,
                    model: Some(model.into()),
                    location: None,
                    current_airport: None,
                })
                .collect();
            save_stored_fleet(
                &mut store,
                &StoredFleetSnapshot {
                    schema_version: FLEET_SNAPSHOT_SCHEMA_VERSION,
                    company: company.clone(),
                    snapshot: Observed {
                        value: aircraft,
                        provenance: Provenance {
                            kind: ProvenanceKind::OnAirFact,
                            source: "onair:company/fleet".into(),
                            observed_at,
                        },
                    },
                },
            )
            .expect("fleet history should save");
        }
        let fbo_observed_at = latest_hour - ChronoDuration::minutes(90);
        save_stored_fbos(
            &mut store,
            &StoredFboSnapshot {
                schema_version: FBOS_SNAPSHOT_SCHEMA_VERSION,
                company: company.clone(),
                snapshot: Observed {
                    value: Vec::new(),
                    provenance: Provenance {
                        kind: ProvenanceKind::OnAirFact,
                        source: "onair:company/fbos".into(),
                        observed_at: fbo_observed_at,
                    },
                },
            },
        )
        .expect("FBO history should save");

        let mut corruptible_store = store.clone();
        let session = OnAirSession::with_store(DEFAULT_BASE_URL, store);
        corruptible_store
            .save_api_snapshot(
                FLEET_RESOURCE_KIND,
                &company.id.0.to_string(),
                &format_timeline_time(latest_hour + ChronoDuration::hours(1)),
                "{\"unsupported\":true}",
            )
            .expect("incompatible historical fixture should save");
        let timeline = session
            .hoard_timeline_index()
            .expect("timeline should be readable");
        assert_eq!(timeline.company, Some(ConnectedCompany::from(&company)));
        assert_eq!(timeline.observation_times.len(), 4);
        assert_eq!(
            timeline
                .fleet_history
                .iter()
                .map(|point| point.aircraft_count)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            timeline.current_fleet_composition,
            vec![
                FleetCompositionPoint {
                    model: "Cessna 172".into(),
                    aircraft_count: 2,
                },
                FleetCompositionPoint {
                    model: "Beechcraft King Air".into(),
                    aircraft_count: 1,
                },
            ]
        );

        let historical = session
            .historical_company_data(&format_timeline_time(
                latest_hour - ChronoDuration::minutes(30),
            ))
            .expect("historical company data should be available");
        assert_eq!(
            historical
                .fleet
                .as_ref()
                .expect("historical fleet should exist")
                .snapshot
                .value
                .len(),
            2
        );
        assert_eq!(
            historical
                .fbos
                .as_ref()
                .expect("historical FBOs should exist")
                .snapshot
                .provenance
                .observed_at,
            fbo_observed_at
        );
        let compatible_fallback = session
            .historical_company_data(&format_timeline_time(
                latest_hour + ChronoDuration::hours(2),
            ))
            .expect("an incompatible record should not hide older compatible history");
        assert_eq!(
            compatible_fallback
                .fleet
                .expect("compatible fleet fallback should exist")
                .snapshot
                .value
                .len(),
            3
        );
        assert!(matches!(
            session.historical_company_data("not-a-time"),
            Err(HoardTimelineError::InvalidSelection)
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_credentials_before_network_access() {
        let session = OnAirSession::default();
        assert!(matches!(
            session.connect("not-a-uuid".into(), "secret".into()).await,
            Err(ConnectionError::InvalidCompanyId)
        ));
        assert!(matches!(
            session.connect(Uuid::nil().to_string(), "  ".into()).await,
            Err(ConnectionError::EmptyApiKey)
        ));
    }

    #[tokio::test]
    async fn refuses_company_sync_without_a_connected_session() {
        let session = OnAirSession::default();
        assert!(matches!(
            session
                .synchronize_company_data(DataSyncTrigger::Manual)
                .await,
            Err(ConnectionError::NotConnected)
        ));
        assert_eq!(
            session
                .fleet_snapshot()
                .expect("snapshot state should be readable"),
            None
        );
        assert_eq!(
            session
                .fbo_snapshot()
                .expect("snapshot state should be readable"),
            None
        );
    }

    #[test]
    fn data_sync_gate_enforces_trigger_specific_quiet_periods() {
        let started = Instant::now();
        let mut gate = DataSyncGate::default();

        assert!(gate.try_start(DataSyncTrigger::Initial, started));
        assert!(!gate.try_start(DataSyncTrigger::Manual, started));
        gate.finish();
        assert!(!gate.try_start(
            DataSyncTrigger::Manual,
            started + MANUAL_SYNC_COOLDOWN - Duration::from_secs(1)
        ));
        assert!(gate.try_start(DataSyncTrigger::Manual, started + MANUAL_SYNC_COOLDOWN));
        gate.finish();
        assert!(!gate.try_start(
            DataSyncTrigger::Automatic,
            started + MANUAL_SYNC_COOLDOWN + Duration::from_secs(1)
        ));
        assert!(gate.try_start(
            DataSyncTrigger::Automatic,
            started + MANUAL_SYNC_COOLDOWN + MINIMUM_AUTOMATIC_SYNC_INTERVAL
        ));
    }

    #[test]
    fn maps_adapter_failures_to_bounded_user_messages() {
        assert!(matches!(
            classify_client_error(ClientError::AuthenticationRejected),
            ConnectionError::AuthenticationRejected
        ));
        assert!(matches!(
            classify_client_error(ClientError::RateLimited),
            ConnectionError::RateLimited
        ));
        assert!(matches!(
            classify_client_error(ClientError::CompanyNotFound),
            ConnectionError::CompanyNotFound
        ));
        let message = ConnectionError::AuthenticationRejected.to_string();
        assert!(message.contains("For now"));
        assert!(message.contains("not OnAir Companion"));
        assert!(matches!(
            classify_client_error(ClientError::MissingContent),
            ConnectionError::ServiceUnavailable
        ));
        assert!(matches!(
            classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fleet),
            ConnectionError::FleetUnavailable
        ));
        assert!(matches!(
            classify_resource_error(ClientError::ApiRejected, CompanyDataResource::Fbos),
            ConnectionError::FbosUnavailable
        ));
    }

    #[test]
    fn exposes_stable_safe_operation_errors() {
        assert_eq!(
            OperationError::from(ConnectionError::RateLimited),
            OperationError {
                code: "onair.rate_limited",
                message: ConnectionError::RateLimited.to_string(),
                retryable: true,
                reportable: false,
                report_id: None,
            }
        );
        assert!(OperationError::from(ConnectionError::StateUnavailable).reportable);
        assert!(!OperationError::from(ConnectionError::AuthenticationRejected).reportable);
    }
}

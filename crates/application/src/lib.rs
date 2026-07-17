//! Application-level orchestration independent of Tauri and other interfaces.

mod authorization;
mod credentials;
mod data_protection;
mod dispatch;
mod display;
mod localization;
mod plugins;
mod simulator;
mod simulator_recording;

pub use authorization::{
    AUTHORIZATION_DECISION_RETENTION_LIMIT, AuthorizationGrantLifetime, AuthorizationRuntime,
    LegalPreferencesRepository, LegalSettingsError, LegalSettingsService, LegalStatus,
    PRIVACY_NOTICE_VERSION, PersistedLegalPreferences, SecurityCentreError,
    SecurityCentreRepository, SecurityCentreService, SecurityCentreStatus, SecurityDecision,
    SecurityDecisionView, SecurityGrantView, SecuritySubjectKind, TERMS_VERSION,
};
pub use credentials::*;
pub use data_protection::*;
pub use dispatch::*;
pub use display::*;
pub use localization::*;
pub use plugins::*;
pub use simulator::*;
pub use simulator_recording::*;

use chrono::{DateTime, SecondsFormat, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_domain::{
    AircraftSummary, CompanyId, CompanySummary, FboSummary, JobSnapshot, Observed, StaffSnapshot,
};
use wyrmgrid_onair_api::{ClientError, DEFAULT_BASE_URL, OnAirClient};
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;
use wyrmgrid_storage::{ApiSnapshotRecord, Store};
use zeroize::Zeroize;

const FLEET_RESOURCE_KIND: &str = "onair_company_fleet";
const FLEET_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const FBOS_RESOURCE_KIND: &str = "onair_company_fbos";
const FBOS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const JOBS_RESOURCE_KIND: &str = "onair_company_pending_jobs";
const JOBS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const STAFF_RESOURCE_KIND: &str = "onair_company_staff";
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<JobSnapshot>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StaffSnapshotView {
    pub company: ConnectedCompany,
    pub snapshot: Observed<StaffSnapshot>,
    pub availability: SnapshotAvailability,
    pub storage: SnapshotStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyDataResource {
    Fleet,
    Fbos,
    Jobs,
    Staff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataSyncFailure {
    pub resource: CompanyDataResource,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompanyDataSyncResult {
    pub disposition: DataSyncDisposition,
    pub fleet: Option<FleetSnapshotView>,
    pub fbos: Option<FboSnapshotView>,
    pub jobs: Option<JobSnapshotView>,
    pub staff: Option<StaffSnapshotView>,
    pub failures: Vec<DataSyncFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetHistoryPoint {
    pub observed_at: String,
    pub aircraft_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FboHistoryPoint {
    pub observed_at: String,
    pub fbo_count: u32,
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
    pub fbo_history: Vec<FboHistoryPoint>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredJobSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<JobSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredStaffSnapshot {
    schema_version: u32,
    company: CompanySummary,
    snapshot: Observed<StaffSnapshot>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
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
    #[error(
        "WyrmGrid could not refresh pending jobs. A previous successful observation, if present, remains available."
    )]
    JobsUnavailable,
    #[error(
        "WyrmGrid could not refresh the staff roster. A previous successful observation, if present, remains available."
    )]
    StaffUnavailable,
    #[error("That pending OnAir job is no longer available in the current observation.")]
    JobUnavailable,
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
            ConnectionError::JobsUnavailable => ("onair.jobs_unavailable", true, false),
            ConnectionError::StaffUnavailable => ("onair.staff_unavailable", true, false),
            ConnectionError::JobUnavailable => ("onair.job_unavailable", false, false),
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

impl From<AccountSettingsError> for OperationError {
    fn from(error: AccountSettingsError) -> Self {
        if let AccountSettingsError::Connection(error) = error {
            return error.into();
        }
        let (code, retryable, reportable) = match error {
            AccountSettingsError::StorageUnavailable => {
                ("accounts.storage_unavailable", true, true)
            }
            AccountSettingsError::CredentialStoreUnavailable => {
                ("accounts.credential_store_unavailable", true, false)
            }
            AccountSettingsError::RememberedSecretMissing => {
                ("accounts.remembered_secret_missing", false, false)
            }
            AccountSettingsError::RememberedAccountMissing => {
                ("accounts.remembered_account_missing", false, false)
            }
            AccountSettingsError::InvalidSimBriefReference => {
                ("simbrief.invalid_user_reference", false, false)
            }
            AccountSettingsError::Connection(_) => unreachable!(),
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

impl From<SecurityCentreError> for OperationError {
    fn from(error: SecurityCentreError) -> Self {
        let code = match error {
            SecurityCentreError::StorageUnavailable => "security.storage_unavailable",
            SecurityCentreError::InvalidRecord => "security.invalid_record",
        };
        Self {
            code,
            message: error.to_string(),
            retryable: matches!(error, SecurityCentreError::StorageUnavailable),
            reportable: true,
            report_id: None,
        }
    }
}

impl From<DataProtectionError> for OperationError {
    fn from(error: DataProtectionError) -> Self {
        let (code, retryable, reportable) = match error {
            DataProtectionError::PasswordTooShort => {
                ("data_protection.password_too_short", false, false)
            }
            DataProtectionError::PasswordTooLong => {
                ("data_protection.password_too_long", false, false)
            }
            DataProtectionError::PasswordConfirmationMismatch => {
                ("data_protection.password_mismatch", false, false)
            }
            DataProtectionError::RestoreConfirmationRequired => (
                "data_protection.restore_confirmation_required",
                false,
                false,
            ),
            DataProtectionError::DestinationExists => {
                ("data_protection.destination_exists", false, false)
            }
            DataProtectionError::InvalidBackup => ("data_protection.invalid_backup", false, false),
            DataProtectionError::SourceIsActiveDatabase => {
                ("data_protection.active_database_selected", false, false)
            }
            DataProtectionError::PersistentStorageRequired => {
                ("data_protection.persistent_storage_required", false, true)
            }
            DataProtectionError::StorageUnavailable => {
                ("data_protection.storage_unavailable", true, true)
            }
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

impl From<DisplaySettingsError> for OperationError {
    fn from(error: DisplaySettingsError) -> Self {
        let (code, retryable) = match error {
            DisplaySettingsError::StorageUnavailable => ("display.storage_unavailable", true),
            DisplaySettingsError::UnsupportedUnit => ("display.unsupported_unit", false),
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

impl From<LanguageSettingsError> for OperationError {
    fn from(error: LanguageSettingsError) -> Self {
        let (code, retryable) = match error {
            LanguageSettingsError::StorageUnavailable => ("language.storage_unavailable", true),
            LanguageSettingsError::ManifestTooLarge => ("language.manifest_too_large", false),
            LanguageSettingsError::InvalidManifest => ("language.invalid_manifest", false),
            LanguageSettingsError::UnsupportedVersion => ("language.unsupported_version", false),
            LanguageSettingsError::InvalidIdentifier => ("language.invalid_identifier", false),
            LanguageSettingsError::InvalidMetadata => ("language.invalid_metadata", false),
            LanguageSettingsError::InvalidMessage => ("language.invalid_message", false),
            LanguageSettingsError::ProtectedMessage => ("language.protected_message", false),
            LanguageSettingsError::UnknownLanguagePack => ("language.unknown_pack", false),
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

impl From<PluginError> for OperationError {
    fn from(error: PluginError) -> Self {
        let (code, retryable, reportable) = match error {
            PluginError::RootUnavailable => ("plugin.root_unavailable", true, true),
            PluginError::StorageUnavailable => ("plugin.storage_unavailable", true, true),
            PluginError::InvalidPlugin => ("plugin.invalid_plugin", false, false),
            PluginError::UnknownPlugin => ("plugin.unknown_plugin", false, false),
            PluginError::UnsupportedRuntime => ("plugin.unsupported_runtime", false, false),
            PluginError::UnsupportedCapability => ("plugin.unsupported_capability", false, false),
            PluginError::PermissionRequired => ("plugin.permission_required", false, false),
            PluginError::AlreadyRunning => ("plugin.already_running", false, false),
            PluginError::NotRunning => ("plugin.not_running", false, false),
            PluginError::RuntimeUnavailable => ("plugin.runtime_unavailable", false, false),
            PluginError::LaunchFailed => ("plugin.launch_failed", true, true),
            PluginError::HandshakeFailed => ("plugin.handshake_failed", true, false),
            PluginError::ProtocolViolation => ("plugin.protocol_violation", false, false),
            PluginError::StateUnavailable => ("plugin.state_unavailable", true, true),
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

impl From<SimulatorBridgeError> for OperationError {
    fn from(error: SimulatorBridgeError) -> Self {
        let (code, retryable, reportable) = match error {
            SimulatorBridgeError::UnknownProvider => ("simulator.unknown_provider", false, false),
            SimulatorBridgeError::InvalidProvider => ("simulator.invalid_provider", false, false),
            SimulatorBridgeError::ProviderUnavailable => {
                ("simulator.provider_unavailable", true, false)
            }
            SimulatorBridgeError::AnotherProviderRunning => {
                ("simulator.another_provider_running", false, false)
            }
            SimulatorBridgeError::AlreadyRunning => {
                ("simulator.provider_already_running", false, false)
            }
            SimulatorBridgeError::NotRunning => ("simulator.provider_not_running", false, false),
            SimulatorBridgeError::LaunchFailed => ("simulator.provider_launch_failed", true, true),
            SimulatorBridgeError::HandshakeFailed => {
                ("simulator.provider_handshake_failed", true, false)
            }
            SimulatorBridgeError::ProtocolViolation => {
                ("simulator.provider_protocol_violation", false, false)
            }
            SimulatorBridgeError::StateUnavailable => ("simulator.state_unavailable", true, true),
            SimulatorBridgeError::PreferencesUnavailable => {
                ("simulator.preferences_unavailable", true, false)
            }
            SimulatorBridgeError::InvalidPreferences => {
                ("simulator.invalid_preferences", false, false)
            }
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

impl From<SimulatorRecordingError> for OperationError {
    fn from(error: SimulatorRecordingError) -> Self {
        let (code, retryable, reportable) = match error {
            SimulatorRecordingError::AlreadyRecording => {
                ("simulator.recording_already_active", false, false)
            }
            SimulatorRecordingError::NotRecording => {
                ("simulator.recording_not_active", false, false)
            }
            SimulatorRecordingError::FreshTelemetryRequired => {
                ("simulator.recording_requires_telemetry", true, false)
            }
            SimulatorRecordingError::InvalidRetention => {
                ("simulator.recording_invalid_retention", false, false)
            }
            SimulatorRecordingError::UnknownSession => {
                ("simulator.recording_unknown_session", false, false)
            }
            SimulatorRecordingError::ExportTooLarge => {
                ("simulator.recording_export_too_large", false, false)
            }
            SimulatorRecordingError::InvalidPlan => {
                ("simulator.recording_invalid_plan", false, false)
            }
            SimulatorRecordingError::ActiveSession => {
                ("simulator.recording_active_session", false, false)
            }
            SimulatorRecordingError::StorageUnavailable => {
                ("simulator.recording_storage_unavailable", true, false)
            }
            SimulatorRecordingError::StateUnavailable => {
                ("simulator.recording_state_unavailable", true, true)
            }
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

impl From<DispatchError> for OperationError {
    fn from(error: DispatchError) -> Self {
        let (code, retryable, reportable) = match error {
            DispatchError::ProviderUnavailable => ("dispatch.provider_unavailable", true, false),
            DispatchError::ImportInProgress => ("dispatch.import_in_progress", true, false),
            DispatchError::WeatherNeedsPlan => ("weather.plan_required", false, false),
            DispatchError::WeatherProviderUnavailable => {
                ("weather.provider_unavailable", true, false)
            }
            DispatchError::WeatherRefreshInProgress => ("weather.refresh_in_progress", true, false),
            DispatchError::WeatherRefreshTooSoon => ("weather.refresh_too_soon", true, false),
            DispatchError::StateUnavailable => ("application.state_unavailable", true, true),
            DispatchError::Provider(wyrmgrid_simbrief_api::ClientError::InvalidUserReference) => {
                ("simbrief.invalid_user_reference", false, false)
            }
            DispatchError::Provider(wyrmgrid_simbrief_api::ClientError::NoPlan) => {
                ("simbrief.no_plan", false, false)
            }
            DispatchError::Provider(wyrmgrid_simbrief_api::ClientError::RateLimited) => {
                ("simbrief.rate_limited", true, false)
            }
            DispatchError::Provider(
                wyrmgrid_simbrief_api::ClientError::TimedOut
                | wyrmgrid_simbrief_api::ClientError::Offline
                | wyrmgrid_simbrief_api::ClientError::ProviderUnavailable,
            ) => ("simbrief.unavailable", true, false),
            DispatchError::Provider(wyrmgrid_simbrief_api::ClientError::ResponseTooLarge) => {
                ("simbrief.response_too_large", false, false)
            }
            DispatchError::Provider(
                wyrmgrid_simbrief_api::ClientError::ConfigurationUnavailable
                | wyrmgrid_simbrief_api::ClientError::UnexpectedResponse
                | wyrmgrid_simbrief_api::ClientError::InvalidContentType
                | wyrmgrid_simbrief_api::ClientError::MalformedPlan,
            ) => ("simbrief.invalid_response", false, true),
            DispatchError::WeatherProvider(wyrmgrid_weather_api::ClientError::InvalidStations) => {
                ("weather.invalid_stations", false, false)
            }
            DispatchError::WeatherProvider(wyrmgrid_weather_api::ClientError::RateLimited) => {
                ("weather.rate_limited", true, false)
            }
            DispatchError::WeatherProvider(
                wyrmgrid_weather_api::ClientError::TimedOut
                | wyrmgrid_weather_api::ClientError::Offline
                | wyrmgrid_weather_api::ClientError::ProviderUnavailable,
            ) => ("weather.unavailable", true, false),
            DispatchError::WeatherProvider(
                wyrmgrid_weather_api::ClientError::ConfigurationUnavailable
                | wyrmgrid_weather_api::ClientError::UnexpectedResponse
                | wyrmgrid_weather_api::ClientError::ResponseTooLarge
                | wyrmgrid_weather_api::ClientError::InvalidContentType
                | wyrmgrid_weather_api::ClientError::MalformedWeather,
            ) => ("weather.invalid_response", false, true),
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

#[derive(Clone)]
pub struct OnAirSession {
    inner: Arc<RwLock<Option<ConnectedSession>>>,
    fleet: Arc<RwLock<Option<FleetSnapshotView>>>,
    fbos: Arc<RwLock<Option<FboSnapshotView>>>,
    jobs: Arc<RwLock<Option<JobSnapshotView>>>,
    staff: Arc<RwLock<Option<StaffSnapshotView>>>,
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
        let mut anchor_company = stored_fleet
            .as_ref()
            .map(|stored| stored.company.id.clone());
        let stored_fbos = load_stored_fbos(&store, anchor_company.as_ref());
        if anchor_company.is_none() {
            anchor_company = stored_fbos.as_ref().map(|stored| stored.company.id.clone());
        }
        let stored_jobs = load_stored_jobs(&store, anchor_company.as_ref());
        if anchor_company.is_none() {
            anchor_company = stored_jobs.as_ref().map(|stored| stored.company.id.clone());
        }
        let stored_staff = load_stored_staff(&store, anchor_company.as_ref());
        let storage = if persistent {
            SnapshotStorage::Hoard
        } else {
            SnapshotStorage::MemoryOnly
        };
        let cached_fleet =
            stored_fleet.map(|stored| fleet_view(stored, SnapshotAvailability::Offline, storage));
        let cached_fbos =
            stored_fbos.map(|stored| fbo_view(stored, SnapshotAvailability::Offline, storage));
        let cached_jobs =
            stored_jobs.map(|stored| job_view(stored, SnapshotAvailability::Offline, storage));
        let cached_staff =
            stored_staff.map(|stored| staff_view(stored, SnapshotAvailability::Offline, storage));
        Self {
            inner: Arc::new(RwLock::new(None)),
            fleet: Arc::new(RwLock::new(cached_fleet)),
            fbos: Arc::new(RwLock::new(cached_fbos)),
            jobs: Arc::new(RwLock::new(cached_jobs)),
            staff: Arc::new(RwLock::new(cached_staff)),
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
        mut api_key: String,
    ) -> Result<ConnectionStatus, ConnectionError> {
        let secret = SecretString::from(api_key.trim().to_owned());
        api_key.zeroize();
        self.connect_secret(company_id, &secret).await
    }

    pub async fn connect_secret(
        &self,
        company_id: String,
        api_key: &SecretString,
    ) -> Result<ConnectionStatus, ConnectionError> {
        let company_id =
            Uuid::parse_str(company_id.trim()).map_err(|_| ConnectionError::InvalidCompanyId)?;
        let api_key = api_key.expose_secret().trim();
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

        let (cached_fleet, cached_fbos, cached_jobs, cached_staff) =
            self.store
                .lock()
                .ok()
                .map_or((None, None, None, None), |store| {
                    let storage = if store.is_persistent() {
                        SnapshotStorage::Hoard
                    } else {
                        SnapshotStorage::MemoryOnly
                    };
                    (
                        load_stored_fleet(&store, Some(&company.id)).map(|stored| {
                            fleet_view(stored, SnapshotAvailability::Cached, storage)
                        }),
                        load_stored_fbos(&store, Some(&company.id))
                            .map(|stored| fbo_view(stored, SnapshotAvailability::Cached, storage)),
                        load_stored_jobs(&store, Some(&company.id))
                            .map(|stored| job_view(stored, SnapshotAvailability::Cached, storage)),
                        load_stored_staff(&store, Some(&company.id)).map(|stored| {
                            staff_view(stored, SnapshotAvailability::Cached, storage)
                        }),
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
        *self
            .jobs
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_jobs;
        *self
            .staff
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = cached_staff;

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
        if let Some(jobs) = self
            .jobs
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            jobs.availability = SnapshotAvailability::Offline;
        }
        if let Some(staff) = self
            .staff
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_mut()
        {
            staff.availability = SnapshotAvailability::Offline;
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
                    jobs: self.job_snapshot()?,
                    staff: self.staff_snapshot()?,
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
                    code: error.diagnostic_code(),
                    message: classify_resource_error(error, CompanyDataResource::Fleet).to_string(),
                });
                self.fleet_snapshot()?
            }
        };

        let mut stop_after_fbos = stop_after_fleet;
        let fbos = if stop_after_fleet {
            self.mark_fbos_cached(&company.id)?;
            failures.push(DataSyncFailure {
                resource: CompanyDataResource::Fbos,
                code: "onair.request_skipped",
                message: "FBO synchronization was skipped to avoid another rejected request."
                    .to_owned(),
            });
            self.fbo_snapshot()?
        } else {
            match client.fbos().await {
                Ok(snapshot) => Some(self.accept_fbo_snapshot(&company, snapshot)?),
                Err(error) => {
                    stop_after_fbos = matches!(
                        error,
                        ClientError::AuthenticationRejected | ClientError::RateLimited
                    );
                    self.mark_fbos_cached(&company.id)?;
                    failures.push(DataSyncFailure {
                        resource: CompanyDataResource::Fbos,
                        code: error.diagnostic_code(),
                        message: classify_resource_error(error, CompanyDataResource::Fbos)
                            .to_string(),
                    });
                    self.fbo_snapshot()?
                }
            }
        };

        let mut stop_after_jobs = stop_after_fbos;
        let jobs = if stop_after_fbos {
            self.mark_jobs_cached(&company.id)?;
            failures.push(DataSyncFailure {
                resource: CompanyDataResource::Jobs,
                code: "onair.request_skipped",
                message:
                    "Pending-job synchronization was skipped to avoid another rejected request."
                        .to_owned(),
            });
            self.job_snapshot()?
        } else {
            match client.pending_jobs().await {
                Ok(snapshot) => Some(self.accept_job_snapshot(&company, snapshot)?),
                Err(error) => {
                    stop_after_jobs = matches!(
                        error,
                        ClientError::AuthenticationRejected | ClientError::RateLimited
                    );
                    self.mark_jobs_cached(&company.id)?;
                    failures.push(DataSyncFailure {
                        resource: CompanyDataResource::Jobs,
                        code: error.diagnostic_code(),
                        message: classify_resource_error(error, CompanyDataResource::Jobs)
                            .to_string(),
                    });
                    self.job_snapshot()?
                }
            }
        };

        let staff = if stop_after_jobs {
            self.mark_staff_cached(&company.id)?;
            failures.push(DataSyncFailure {
                resource: CompanyDataResource::Staff,
                code: "onair.request_skipped",
                message: "Staff synchronization was skipped to avoid another rejected request."
                    .to_owned(),
            });
            self.staff_snapshot()?
        } else {
            match client.staff().await {
                Ok(snapshot) => Some(self.accept_staff_snapshot(&company, snapshot)?),
                Err(error) => {
                    self.mark_staff_cached(&company.id)?;
                    failures.push(DataSyncFailure {
                        resource: CompanyDataResource::Staff,
                        code: error.diagnostic_code(),
                        message: classify_resource_error(error, CompanyDataResource::Staff)
                            .to_string(),
                    });
                    self.staff_snapshot()?
                }
            }
        };

        Ok(CompanyDataSyncResult {
            disposition: DataSyncDisposition::Synchronized,
            fleet,
            fbos,
            jobs,
            staff,
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

    fn accept_job_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<JobSnapshot>,
    ) -> Result<JobSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        snapshot
            .value
            .validate()
            .map_err(|_| ConnectionError::JobsUnavailable)?;
        let stored = StoredJobSnapshot {
            schema_version: JOBS_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_jobs(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = job_view(stored, SnapshotAvailability::Live, storage);
        *self
            .jobs
            .write()
            .map_err(|_| ConnectionError::StateUnavailable)? = Some(view.clone());
        Ok(view)
    }

    fn accept_staff_snapshot(
        &self,
        company: &CompanySummary,
        snapshot: Observed<StaffSnapshot>,
    ) -> Result<StaffSnapshotView, ConnectionError> {
        self.ensure_current_company(&company.id)?;
        snapshot
            .value
            .validate()
            .map_err(|_| ConnectionError::StaffUnavailable)?;
        let stored = StoredStaffSnapshot {
            schema_version: wyrmgrid_domain::STAFF_SNAPSHOT_SCHEMA_VERSION,
            company: company.clone(),
            snapshot,
        };
        let storage = self
            .store
            .lock()
            .ok()
            .filter(|store| store.is_persistent())
            .and_then(|mut store| save_stored_staff(&mut store, &stored).ok())
            .map_or(SnapshotStorage::MemoryOnly, |_| SnapshotStorage::Hoard);
        let view = staff_view(stored, SnapshotAvailability::Live, storage);
        *self
            .staff
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

    pub fn job_snapshot(&self) -> Result<Option<JobSnapshotView>, ConnectionError> {
        self.jobs
            .read()
            .map(|jobs| jobs.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    pub fn staff_snapshot(&self) -> Result<Option<StaffSnapshotView>, ConnectionError> {
        self.staff
            .read()
            .map(|staff| staff.clone())
            .map_err(|_| ConnectionError::StateUnavailable)
    }

    pub fn job_for_dispatch(&self, job_id: &str) -> Result<DispatchJobSelection, ConnectionError> {
        let job_id = Uuid::parse_str(job_id).map_err(|_| ConnectionError::JobUnavailable)?;
        let jobs = self
            .jobs
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?;
        let view = jobs.as_ref().ok_or(ConnectionError::JobUnavailable)?;
        let job = view
            .snapshot
            .value
            .jobs
            .iter()
            .find(|job| job.id.0 == job_id)
            .cloned()
            .ok_or(ConnectionError::JobUnavailable)?;
        Ok(DispatchJobSelection {
            job,
            observed_at: view.snapshot.provenance.observed_at,
            availability: view.availability,
        })
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
                fbo_history: Vec::new(),
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
        let fbo_history = valid_fbos
            .iter()
            .map(|stored| FboHistoryPoint {
                observed_at: format_timeline_time(stored.snapshot.provenance.observed_at),
                fbo_count: bounded_count(stored.snapshot.value.len()),
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
            fbo_history,
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

    fn mark_jobs_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(jobs) = self
                .jobs
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            jobs.availability = SnapshotAvailability::Cached;
        }
        Ok(())
    }

    fn mark_staff_cached(&self, company_id: &CompanyId) -> Result<(), ConnectionError> {
        let is_current_company = self
            .inner
            .read()
            .map_err(|_| ConnectionError::StateUnavailable)?
            .as_ref()
            .is_some_and(|connected| &connected.company.id == company_id);
        if is_current_company
            && let Some(staff) = self
                .staff
                .write()
                .map_err(|_| ConnectionError::StateUnavailable)?
                .as_mut()
        {
            staff.availability = SnapshotAvailability::Cached;
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

fn job_view(
    stored: StoredJobSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> JobSnapshotView {
    JobSnapshotView {
        company: ConnectedCompany::from(&stored.company),
        snapshot: stored.snapshot,
        availability,
        storage,
    }
}

fn staff_view(
    stored: StoredStaffSnapshot,
    availability: SnapshotAvailability,
    storage: SnapshotStorage,
) -> StaffSnapshotView {
    StaffSnapshotView {
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

fn load_stored_jobs(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredJobSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(JOBS_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    stored_jobs_from_record(record)
}

fn stored_jobs_from_record(record: ApiSnapshotRecord) -> Option<StoredJobSnapshot> {
    let stored: StoredJobSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == JOBS_SNAPSHOT_SCHEMA_VERSION
        && stored.snapshot.value.validate().is_ok()
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_jobs(store: &mut Store, stored: &StoredJobSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            JOBS_RESOURCE_KIND,
            &stored.company.id.0.to_string(),
            &stored.snapshot.provenance.observed_at.to_rfc3339(),
            &payload,
        )
        .map_err(|_| ())
}

fn load_stored_staff(store: &Store, company_id: Option<&CompanyId>) -> Option<StoredStaffSnapshot> {
    let resource_key = company_id.map(|id| id.0.to_string());
    let record = store
        .latest_api_snapshot(STAFF_RESOURCE_KIND, resource_key.as_deref())
        .ok()??;
    stored_staff_from_record(record)
}

fn stored_staff_from_record(record: ApiSnapshotRecord) -> Option<StoredStaffSnapshot> {
    let stored: StoredStaffSnapshot = serde_json::from_str(&record.payload_json).ok()?;
    (stored.schema_version == wyrmgrid_domain::STAFF_SNAPSHOT_SCHEMA_VERSION
        && stored.snapshot.value.validate().is_ok()
        && record.resource_key == stored.company.id.0.to_string())
    .then_some(stored)
}

fn save_stored_staff(store: &mut Store, stored: &StoredStaffSnapshot) -> Result<(), ()> {
    let payload = serde_json::to_string(stored).map_err(|_| ())?;
    store
        .save_api_snapshot(
            STAFF_RESOURCE_KIND,
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
            CompanyDataResource::Jobs => ConnectionError::JobsUnavailable,
            CompanyDataResource::Staff => ConnectionError::StaffUnavailable,
        },
    }
}

#[cfg(test)]
#[path = "tests/application.rs"]
mod tests;

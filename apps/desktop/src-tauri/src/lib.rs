mod credential_store;
mod database_key;
mod diagnostics;
mod observability;

use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use zeroize::Zeroize;

#[derive(Clone, Debug, Default, serde::Serialize)]
struct StartupOptions {
    no_launch_art: bool,
    compact_ui: bool,
    low_resource: bool,
}

struct DesktopState {
    startup_options: StartupOptions,
    onair: wyrmgrid_application::OnAirSession,
    accounts: wyrmgrid_application::AccountSettingsService<
        wyrmgrid_storage::Store,
        credential_store::PlatformOnAirSecretStore,
    >,
    dispatch: wyrmgrid_application::DispatchSession,
    plugins: wyrmgrid_application::PluginService,
    simulator: wyrmgrid_application::SimulatorBridgeService,
    simulator_settings: wyrmgrid_application::SimulatorSettingsService<wyrmgrid_storage::Store>,
    simulator_recording: wyrmgrid_application::SimulatorRecordingService,
    legal: wyrmgrid_application::LegalSettingsService<wyrmgrid_storage::Store>,
    security: wyrmgrid_application::SecurityCentreService<wyrmgrid_storage::Store>,
    data_protection: wyrmgrid_application::DataProtectionService<wyrmgrid_storage::Store>,
    device_keys: database_key::DeviceKeyStore,
    themes: wyrmgrid_application::ThemeSettingsService<wyrmgrid_storage::Store>,
    languages: wyrmgrid_application::LanguageSettingsService<wyrmgrid_storage::Store>,
    display: wyrmgrid_application::DisplaySettingsService<wyrmgrid_storage::Store>,
    observability: observability::Controller,
}

#[tauri::command]
fn startup_options(state: tauri::State<'_, DesktopState>) -> StartupOptions {
    state.startup_options.clone()
}

#[tauri::command]
fn platform_status() -> wyrmgrid_application::PlatformStatus {
    wyrmgrid_application::platform_status()
}

#[tauri::command]
fn onair_connection_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::ConnectionStatus, wyrmgrid_application::OperationError> {
    state.onair.status().map_err(operation_error)
}

#[tauri::command]
async fn connect_onair(
    state: tauri::State<'_, DesktopState>,
    company_id: String,
    mut api_key: String,
    remember: bool,
    connect_on_start: bool,
) -> Result<wyrmgrid_application::OnAirConnectionResult, wyrmgrid_application::OperationError> {
    let result = state
        .accounts
        .connect(
            company_id,
            std::mem::take(&mut api_key),
            remember,
            connect_on_start,
        )
        .await
        .map_err(operation_error);
    api_key.zeroize();
    result
}

#[tauri::command]
fn onair_credential_profile_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::OnAirCredentialProfileStatus, wyrmgrid_application::OperationError>
{
    state.accounts.onair_status().map_err(operation_error)
}

#[tauri::command]
async fn connect_remembered_onair(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::OnAirConnectionResult, wyrmgrid_application::OperationError> {
    state
        .accounts
        .connect_remembered()
        .await
        .map_err(operation_error)
}

#[tauri::command]
async fn auto_connect_onair_if_enabled(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::OnAirConnectionResult>, wyrmgrid_application::OperationError>
{
    state
        .accounts
        .connect_on_start_if_enabled()
        .await
        .map_err(operation_error)
}

#[tauri::command]
fn forget_onair_credentials(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::OnAirCredentialProfileStatus, wyrmgrid_application::OperationError>
{
    state.accounts.forget_onair().map_err(operation_error)
}

#[tauri::command]
fn disconnect_onair(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::ConnectionStatus, wyrmgrid_application::OperationError> {
    state.onair.disconnect().map_err(operation_error)
}

#[tauri::command]
async fn synchronize_onair_company_data(
    state: tauri::State<'_, DesktopState>,
    trigger: wyrmgrid_application::DataSyncTrigger,
) -> Result<wyrmgrid_application::CompanyDataSyncResult, wyrmgrid_application::OperationError> {
    let result = state
        .onair
        .synchronize_company_data(trigger)
        .await
        .map_err(operation_error)?;
    for failure in &result.failures {
        diagnostics::record(
            if failure.code == "onair.request_skipped" {
                "warning"
            } else {
                "error"
            },
            failure.code,
            "synchronize_onair_company_data",
            &failure.message,
        );
    }
    Ok(result)
}

#[tauri::command]
fn diagnostic_log() -> diagnostics::DiagnosticLogView {
    diagnostics::view()
}

#[tauri::command]
fn clear_diagnostic_log() -> diagnostics::DiagnosticLogView {
    diagnostics::clear()
}

#[tauri::command]
fn onair_fleet_snapshot(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::FleetSnapshotView>, wyrmgrid_application::OperationError> {
    state.onair.fleet_snapshot().map_err(operation_error)
}

#[tauri::command]
fn onair_fbo_snapshot(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::FboSnapshotView>, wyrmgrid_application::OperationError> {
    state.onair.fbo_snapshot().map_err(operation_error)
}

#[tauri::command]
fn onair_job_snapshot(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::JobSnapshotView>, wyrmgrid_application::OperationError> {
    state.onair.job_snapshot().map_err(operation_error)
}

#[tauri::command]
fn onair_hoard_timeline(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::HoardTimelineIndex, wyrmgrid_application::OperationError> {
    state.onair.hoard_timeline_index().map_err(operation_error)
}

#[tauri::command]
fn onair_historical_company_data(
    state: tauri::State<'_, DesktopState>,
    selected_at: String,
) -> Result<wyrmgrid_application::HistoricalCompanyDataView, wyrmgrid_application::OperationError> {
    state
        .onair
        .historical_company_data(&selected_at)
        .map_err(operation_error)
}

#[tauri::command]
fn dispatch_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    let fleet = state.onair.fleet_snapshot().map_err(operation_error)?;
    state
        .dispatch
        .briefing(fleet.as_ref())
        .map_err(operation_error)
}

#[tauri::command]
fn select_dispatch_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    let selection = state
        .onair
        .job_for_dispatch(&job_id)
        .map_err(operation_error)?;
    state
        .dispatch
        .select_job(selection)
        .map_err(operation_error)?;
    dispatch_status(state)
}

#[tauri::command]
fn clear_dispatch_job(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    state.dispatch.clear_job().map_err(operation_error)?;
    dispatch_status(state)
}

#[tauri::command]
async fn import_simbrief_latest(
    state: tauri::State<'_, DesktopState>,
    reference_kind: wyrmgrid_application::SimBriefReferenceKind,
    reference: String,
    remember_reference: bool,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    state
        .dispatch
        .import_latest(reference_kind, &reference)
        .await
        .map_err(operation_error)?;
    state
        .accounts
        .remember_simbrief(reference_kind, &reference, remember_reference)
        .map_err(operation_error)?;
    let status = dispatch_status(state.clone())?;
    state
        .simulator_recording
        .set_plan_context(status.snapshot.clone())
        .map_err(operation_error)?;
    Ok(status)
}

#[tauri::command]
fn simbrief_account_preference(
    state: tauri::State<'_, DesktopState>,
) -> Result<
    Option<wyrmgrid_application::SimBriefAccountPreference>,
    wyrmgrid_application::OperationError,
> {
    state.accounts.simbrief_status().map_err(operation_error)
}

#[tauri::command]
async fn refresh_dispatch_weather(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    state
        .dispatch
        .refresh_weather()
        .await
        .map_err(operation_error)?;
    dispatch_status(state)
}

#[tauri::command]
fn clear_dispatch_plan(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    state.dispatch.clear().map_err(operation_error)?;
    state
        .simulator_recording
        .set_plan_context(None)
        .map_err(operation_error)?;
    dispatch_status(state)
}

#[tauri::command]
fn legal_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::LegalStatus, wyrmgrid_application::OperationError> {
    state.legal.status().map_err(operation_error)
}

#[tauri::command]
fn security_centre_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SecurityCentreStatus, wyrmgrid_application::OperationError> {
    state.security.status().map_err(operation_error)
}

#[tauri::command]
fn data_protection_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DataProtectionStatus, wyrmgrid_application::OperationError> {
    state.data_protection.status().map_err(operation_error)
}

#[tauri::command]
async fn create_portable_backup(
    state: tauri::State<'_, DesktopState>,
    destination: String,
    mut password: String,
    mut password_confirmation: String,
) -> Result<wyrmgrid_application::PortableBackupView, wyrmgrid_application::OperationError> {
    let data_protection = state.data_protection.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = data_protection.create_portable_backup(
            std::path::Path::new(&destination),
            &password,
            &password_confirmation,
        );
        password.zeroize();
        password_confirmation.zeroize();
        result
    })
    .await
    .map_err(|_| operation_error(wyrmgrid_application::DataProtectionError::StorageUnavailable))?
    .map_err(operation_error)
}

#[tauri::command]
async fn prepare_portable_restore(
    state: tauri::State<'_, DesktopState>,
    source: String,
    mut password: String,
    replacement_confirmed: bool,
) -> Result<wyrmgrid_application::PortableRestoreView, wyrmgrid_application::OperationError> {
    let data_protection = state.data_protection.clone();
    let device_keys = state.device_keys.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = device_keys
            .load_existing()
            .map_err(|_| wyrmgrid_application::DataProtectionError::StorageUnavailable)
            .and_then(|device_key| {
                data_protection.prepare_portable_restore(
                    std::path::Path::new(&source),
                    &password,
                    replacement_confirmed,
                    &device_key,
                )
            });
        password.zeroize();
        result
    })
    .await
    .map_err(|_| operation_error(wyrmgrid_application::DataProtectionError::StorageUnavailable))?
    .map_err(operation_error)
}

#[tauri::command]
fn acknowledge_legal(
    state: tauri::State<'_, DesktopState>,
    telemetry_enabled: bool,
) -> Result<wyrmgrid_application::LegalStatus, wyrmgrid_application::OperationError> {
    let status = state
        .legal
        .acknowledge(telemetry_enabled)
        .map_err(operation_error)?;
    state
        .observability
        .apply_user_preference(status.telemetry_enabled);
    Ok(status)
}

#[tauri::command]
fn update_telemetry_preference(
    state: tauri::State<'_, DesktopState>,
    telemetry_enabled: bool,
) -> Result<wyrmgrid_application::LegalStatus, wyrmgrid_application::OperationError> {
    let status = state
        .legal
        .update_telemetry(telemetry_enabled)
        .map_err(operation_error)?;
    state
        .observability
        .apply_user_preference(status.telemetry_enabled);
    Ok(status)
}

#[tauri::command]
fn theme_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::ThemeStatus, wyrmgrid_application::OperationError> {
    state.themes.status().map_err(operation_error)
}

#[tauri::command]
fn display_preferences(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::DisplayPreferences, wyrmgrid_application::OperationError> {
    state.display.status().map_err(operation_error)
}

#[tauri::command]
fn update_display_preferences(
    state: tauri::State<'_, DesktopState>,
    preferences: wyrmgrid_application::DisplayPreferences,
) -> Result<wyrmgrid_application::DisplayPreferences, wyrmgrid_application::OperationError> {
    state.display.update(preferences).map_err(operation_error)
}

#[tauri::command]
fn select_theme(
    state: tauri::State<'_, DesktopState>,
    theme_id: String,
) -> Result<wyrmgrid_application::ThemeStatus, wyrmgrid_application::OperationError> {
    state.themes.select(&theme_id).map_err(operation_error)
}

#[tauri::command]
fn import_theme(
    state: tauri::State<'_, DesktopState>,
    manifest_json: String,
) -> Result<wyrmgrid_application::ThemeStatus, wyrmgrid_application::OperationError> {
    state.themes.import(&manifest_json).map_err(operation_error)
}

#[tauri::command]
fn language_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::LanguageStatus, wyrmgrid_application::OperationError> {
    state.languages.status().map_err(operation_error)
}

#[tauri::command]
fn select_language_pack(
    state: tauri::State<'_, DesktopState>,
    pack_id: String,
) -> Result<wyrmgrid_application::LanguageStatus, wyrmgrid_application::OperationError> {
    state.languages.select(&pack_id).map_err(operation_error)
}

#[tauri::command]
fn import_language_pack(
    state: tauri::State<'_, DesktopState>,
    manifest_json: String,
) -> Result<wyrmgrid_application::LanguageStatus, wyrmgrid_application::OperationError> {
    state
        .languages
        .import(&manifest_json)
        .map_err(operation_error)
}

#[tauri::command]
fn plugin_host_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    state.plugins.status().map_err(operation_error)
}

#[tauri::command]
fn approve_plugin_permissions(
    state: tauri::State<'_, DesktopState>,
    plugin_id: String,
    lifetime: wyrmgrid_application::AuthorizationGrantLifetime,
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    state
        .plugins
        .approve_requested_permissions_with_lifetime(&plugin_id, lifetime)
        .map_err(operation_error)
}

#[tauri::command]
async fn revoke_plugin_permissions(
    state: tauri::State<'_, DesktopState>,
    plugin_id: String,
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    let plugins = state.plugins.clone();
    tauri::async_runtime::spawn_blocking(move || plugins.revoke_permissions(&plugin_id))
        .await
        .map_err(|_| operation_error(wyrmgrid_application::PluginError::StateUnavailable))?
        .map_err(operation_error)
}

#[tauri::command]
async fn start_plugin(
    state: tauri::State<'_, DesktopState>,
    plugin_id: String,
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    let plugins = state.plugins.clone();
    tauri::async_runtime::spawn_blocking(move || plugins.start(&plugin_id))
        .await
        .map_err(|_| operation_error(wyrmgrid_application::PluginError::StateUnavailable))?
        .map_err(operation_error)
}

#[tauri::command]
async fn stop_plugin(
    state: tauri::State<'_, DesktopState>,
    plugin_id: String,
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    let plugins = state.plugins.clone();
    tauri::async_runtime::spawn_blocking(move || plugins.stop(&plugin_id))
        .await
        .map_err(|_| operation_error(wyrmgrid_application::PluginError::StateUnavailable))?
        .map_err(operation_error)
}

#[tauri::command]
fn simulator_bridge_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorBridgeView, wyrmgrid_application::OperationError> {
    state.simulator.status().map_err(operation_error)
}

#[tauri::command]
fn simulator_preferences(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorPreferences, wyrmgrid_application::OperationError> {
    state.simulator_settings.status().map_err(operation_error)
}

#[tauri::command]
fn update_simulator_preferences(
    state: tauri::State<'_, DesktopState>,
    preferences: wyrmgrid_application::SimulatorPreferences,
) -> Result<wyrmgrid_application::SimulatorPreferences, wyrmgrid_application::OperationError> {
    state
        .simulator_settings
        .update(preferences)
        .map_err(operation_error)
}

#[tauri::command]
async fn start_simulator_provider(
    state: tauri::State<'_, DesktopState>,
    provider_id: String,
) -> Result<wyrmgrid_application::SimulatorBridgeView, wyrmgrid_application::OperationError> {
    state
        .simulator_settings
        .select_provider(&provider_id)
        .map_err(operation_error)?;
    let simulator = state.simulator.clone();
    tauri::async_runtime::spawn_blocking(move || simulator.start(&provider_id))
        .await
        .map_err(|_| operation_error(wyrmgrid_application::SimulatorBridgeError::StateUnavailable))?
        .map_err(operation_error)
}

#[tauri::command]
async fn stop_simulator_provider(
    state: tauri::State<'_, DesktopState>,
    provider_id: String,
) -> Result<wyrmgrid_application::SimulatorBridgeView, wyrmgrid_application::OperationError> {
    let simulator = state.simulator.clone();
    tauri::async_runtime::spawn_blocking(move || simulator.stop(&provider_id))
        .await
        .map_err(|_| operation_error(wyrmgrid_application::SimulatorBridgeError::StateUnavailable))?
        .map_err(operation_error)
}

#[tauri::command]
fn simulator_recording_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    state.simulator_recording.status().map_err(operation_error)
}

#[tauri::command]
fn update_simulator_recording_preferences(
    state: tauri::State<'_, DesktopState>,
    preferences: wyrmgrid_application::SimulatorRecordingPreferences,
) -> Result<wyrmgrid_application::SimulatorRecordingPreferences, wyrmgrid_application::OperationError>
{
    state
        .simulator_recording
        .update_preferences(preferences)
        .map_err(operation_error)
}

#[tauri::command]
fn start_simulator_recording(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    let bridge = state.simulator.status().map_err(operation_error)?;
    let provider_id = bridge.active_provider_id.ok_or_else(|| {
        operation_error(wyrmgrid_application::SimulatorRecordingError::FreshTelemetryRequired)
    })?;
    state
        .simulator_recording
        .start(&provider_id, bridge.latest_snapshot)
        .map_err(operation_error)
}

#[tauri::command]
fn stop_simulator_recording(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    state.simulator_recording.stop().map_err(operation_error)
}

#[tauri::command]
fn simulator_recording_session(
    state: tauri::State<'_, DesktopState>,
    session_id: String,
    sample_offset: Option<u32>,
) -> Result<wyrmgrid_application::SimulatorSessionView, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .session_window(&session_id, sample_offset.unwrap_or(0))
        .map_err(operation_error)
}

#[tauri::command]
fn simulator_recording_debrief(
    state: tauri::State<'_, DesktopState>,
    session_id: String,
) -> Result<wyrmgrid_application::SimulatorSessionDebrief, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .debrief(&session_id)
        .map_err(operation_error)
}

#[tauri::command]
fn pin_simulator_recording(
    state: tauri::State<'_, DesktopState>,
    session_id: String,
    pinned: bool,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .set_pinned(&session_id, pinned)
        .map_err(operation_error)
}

#[tauri::command]
fn export_simulator_recording(
    state: tauri::State<'_, DesktopState>,
    session_id: String,
    format: wyrmgrid_application::SimulatorExportFormat,
) -> Result<wyrmgrid_application::SimulatorRecordingExport, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .export_session(&session_id, format)
        .map_err(operation_error)
}

#[tauri::command]
fn delete_simulator_recording(
    state: tauri::State<'_, DesktopState>,
    session_id: String,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .delete_session(&session_id)
        .map_err(operation_error)
}

#[tauri::command]
fn delete_all_simulator_recordings(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::SimulatorRecordingView, wyrmgrid_application::OperationError> {
    state
        .simulator_recording
        .delete_all()
        .map_err(operation_error)
}

fn operation_error<E: Into<wyrmgrid_application::OperationError>>(
    error: E,
) -> wyrmgrid_application::OperationError {
    let operation_error = error.into();
    diagnostics::record(
        "error",
        operation_error.code,
        "desktop_command",
        &operation_error.message,
    );
    if operation_error.reportable {
        let report_id = observability::capture_reportable(operation_error.code);
        operation_error.with_report_id(report_id)
    } else {
        operation_error
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let parsed_startup_options = parse_startup_options(std::env::args_os().skip(1));
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_data_directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_directory)?;
            diagnostics::initialize(Some(&app_data_directory));
            let database_path = app_data_directory.join("wyrmgrid.db");
            let device_keys = database_key::DeviceKeyStore;
            let database_key = device_keys
                .load_or_create(wyrmgrid_storage::encrypted_database_state_exists(
                    &database_path,
                ))
                .inspect_err(|_| {
                    show_encrypted_storage_startup_error(app);
                })?;
            let store =
                wyrmgrid_storage::Store::open(&database_path, &database_key).inspect_err(|_| {
                    show_encrypted_storage_startup_error(app);
                })?;
            let legal = wyrmgrid_application::LegalSettingsService::new(store.clone());
            let legal_status = legal.status().expect("legal settings should initialize");
            let themes = wyrmgrid_application::ThemeSettingsService::new(store.clone());
            let languages = wyrmgrid_application::LanguageSettingsService::new(store.clone());
            let display = wyrmgrid_application::DisplaySettingsService::new(store.clone());
            let authorization_runtime = wyrmgrid_application::AuthorizationRuntime::default();
            let security = wyrmgrid_application::SecurityCentreService::with_runtime(
                store.clone(),
                authorization_runtime.clone(),
            );
            let data_protection = wyrmgrid_application::DataProtectionService::new(store.clone());
            let onair = wyrmgrid_application::OnAirSession::with_default_store(store.clone());
            let accounts = wyrmgrid_application::AccountSettingsService::new(
                store.clone(),
                credential_store::PlatformOnAirSecretStore,
                onair.clone(),
            );
            let dispatch = wyrmgrid_application::DispatchSession::with_default_provider();
            let simulator_provider =
                wyrmgrid_application::SimulatorProviderRegistration::from_manifest_json(
                    include_str!("../../../../providers/msfs2024-simconnect/provider.json"),
                    simulator_provider_path(),
                )
                .expect("bundled simulator provider manifest should validate");
            let simulator_recording =
                wyrmgrid_application::SimulatorRecordingService::new(store.clone());
            let simulator = wyrmgrid_application::SimulatorBridgeService::with_telemetry_observer(
                vec![simulator_provider],
                Some(Arc::new(simulator_recording.clone())),
            );
            let simulator_settings = wyrmgrid_application::SimulatorSettingsService::new(
                store.clone(),
                simulator.provider_ids(),
            );
            let automatic_provider = simulator_settings.startup_provider_id().ok().flatten();
            let plugins = wyrmgrid_application::PluginService::with_authorization_runtime(
                Some(app_data_directory.join("plugins")),
                store,
                onair.clone(),
                simulator.clone(),
                authorization_runtime,
            );

            app.manage(DesktopState {
                startup_options: parsed_startup_options.clone(),
                onair,
                accounts,
                dispatch,
                plugins,
                simulator,
                simulator_settings,
                simulator_recording,
                legal,
                security,
                data_protection,
                device_keys,
                themes,
                languages,
                display,
                observability: observability::Controller::new(legal_status.telemetry_enabled),
            });
            if let Some(provider_id) = automatic_provider {
                let simulator = app.state::<DesktopState>().simulator.clone();
                std::thread::spawn(move || {
                    if let Err(error) = simulator.start(&provider_id) {
                        diagnostics::record(
                            "warning",
                            "simulator.automatic_start_failed",
                            "desktop_startup",
                            &error.to_string(),
                        );
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup_options,
            platform_status,
            onair_connection_status,
            onair_credential_profile_status,
            connect_onair,
            connect_remembered_onair,
            auto_connect_onair_if_enabled,
            forget_onair_credentials,
            disconnect_onair,
            synchronize_onair_company_data,
            onair_fleet_snapshot,
            onair_fbo_snapshot,
            onair_job_snapshot,
            onair_hoard_timeline,
            onair_historical_company_data,
            dispatch_status,
            select_dispatch_job,
            clear_dispatch_job,
            import_simbrief_latest,
            simbrief_account_preference,
            refresh_dispatch_weather,
            clear_dispatch_plan,
            diagnostic_log,
            clear_diagnostic_log,
            legal_status,
            acknowledge_legal,
            update_telemetry_preference,
            security_centre_status,
            data_protection_status,
            create_portable_backup,
            prepare_portable_restore,
            theme_status,
            display_preferences,
            update_display_preferences,
            select_theme,
            import_theme,
            language_status,
            select_language_pack,
            import_language_pack,
            plugin_host_status,
            approve_plugin_permissions,
            revoke_plugin_permissions,
            start_plugin,
            stop_plugin,
            simulator_bridge_status,
            simulator_preferences,
            update_simulator_preferences,
            start_simulator_provider,
            stop_simulator_provider,
            simulator_recording_status,
            update_simulator_recording_preferences,
            start_simulator_recording,
            stop_simulator_recording,
            simulator_recording_session,
            simulator_recording_debrief,
            pin_simulator_recording,
            export_simulator_recording,
            delete_simulator_recording,
            delete_all_simulator_recordings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_encrypted_storage_startup_error(app: &tauri::App) {
    app.dialog()
        .message(
            "WyrmGrid could not unlock its encrypted local data. The operating-system credential may be unavailable, or the database may not match it. WyrmGrid did not replace or open the data as plaintext. Recover the original device credential or restore a portable WyrmGrid backup.",
        )
        .title("Encrypted WyrmGrid data unavailable")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

fn parse_startup_options<I, S>(arguments: I) -> StartupOptions
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut options = StartupOptions::default();
    for argument in arguments {
        match argument.as_ref().to_string_lossy().as_ref() {
            "--no-launch-art" => options.no_launch_art = true,
            "--compact-ui" => options.compact_ui = true,
            "--low-resource" => options.low_resource = true,
            _ => {}
        }
    }
    if options.low_resource {
        options.no_launch_art = true;
        options.compact_ui = true;
    }
    options
}

fn simulator_provider_path() -> std::path::PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    resolve_simulator_provider_path(
        std::env::var_os("WYRMGRID_SIMULATOR_PROVIDER_PATH"),
        std::env::current_exe().ok(),
        &workspace_root,
        cfg!(debug_assertions),
    )
}

const SIMULATOR_PROVIDER_EXECUTABLE: &str = "wyrmgrid-simconnect-provider.exe";

fn resolve_simulator_provider_path(
    configured: Option<std::ffi::OsString>,
    current_executable: Option<std::path::PathBuf>,
    workspace_root: &std::path::Path,
    development_mode: bool,
) -> std::path::PathBuf {
    if let Some(path) = configured {
        let path = std::path::PathBuf::from(path);
        if path.is_absolute()
            && path.file_name().and_then(|name| name.to_str())
                == Some(SIMULATOR_PROVIDER_EXECUTABLE)
        {
            return path;
        }
    }
    if let Some(directory) = current_executable
        .as_deref()
        .and_then(std::path::Path::parent)
    {
        let adjacent = directory.join(SIMULATOR_PROVIDER_EXECUTABLE);
        if adjacent.is_file() {
            return adjacent;
        }
    }
    if development_mode {
        let development = workspace_root
            .join("target/debug")
            .join(SIMULATOR_PROVIDER_EXECUTABLE);
        if development.is_file() {
            return development;
        }
        return development;
    }
    current_executable
        .as_deref()
        .and_then(std::path::Path::parent)
        .map(|directory| directory.join(SIMULATOR_PROVIDER_EXECUTABLE))
        .unwrap_or_else(|| std::path::PathBuf::from(SIMULATOR_PROVIDER_EXECUTABLE))
}

#[cfg(test)]
#[path = "tests/simulator_provider.rs"]
mod simulator_provider_tests;

#[cfg(test)]
#[path = "tests/startup_options.rs"]
mod startup_options_tests;

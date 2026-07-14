mod diagnostics;
mod observability;

use tauri::Manager;

#[derive(Clone, Debug, Default, serde::Serialize)]
struct StartupOptions {
    no_launch_art: bool,
    compact_ui: bool,
    low_resource: bool,
}

struct DesktopState {
    startup_options: StartupOptions,
    onair: wyrmgrid_application::OnAirSession,
    dispatch: wyrmgrid_application::DispatchSession,
    plugins: wyrmgrid_application::PluginService,
    simulator: wyrmgrid_application::SimulatorBridgeService,
    legal: wyrmgrid_application::LegalSettingsService<wyrmgrid_storage::Store>,
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
    api_key: String,
) -> Result<wyrmgrid_application::ConnectionStatus, wyrmgrid_application::OperationError> {
    state
        .onair
        .connect(company_id, api_key)
        .await
        .map_err(operation_error)
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
) -> Result<wyrmgrid_application::DispatchStatus, wyrmgrid_application::OperationError> {
    state
        .dispatch
        .import_latest(reference_kind, &reference)
        .await
        .map_err(operation_error)?;
    dispatch_status(state)
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
    dispatch_status(state)
}

#[tauri::command]
fn legal_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::LegalStatus, wyrmgrid_application::OperationError> {
    state.legal.status().map_err(operation_error)
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
) -> Result<wyrmgrid_application::PluginHostView, wyrmgrid_application::OperationError> {
    state
        .plugins
        .approve_requested_permissions(&plugin_id)
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
async fn start_simulator_provider(
    state: tauri::State<'_, DesktopState>,
    provider_id: String,
) -> Result<wyrmgrid_application::SimulatorBridgeView, wyrmgrid_application::OperationError> {
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
        .setup(move |app| {
            let app_data_directory =
                app.path().app_data_dir().ok().and_then(|directory| {
                    std::fs::create_dir_all(&directory).ok().map(|_| directory)
                });
            diagnostics::initialize(app_data_directory.as_deref());
            let store = app_data_directory
                .as_ref()
                .and_then(|directory| {
                    wyrmgrid_storage::Store::open(directory.join("wyrmgrid.db")).ok()
                })
                .unwrap_or_else(|| {
                    wyrmgrid_storage::Store::open_in_memory()
                        .expect("in-memory Hoard fallback should initialize")
                });
            let legal = wyrmgrid_application::LegalSettingsService::new(store.clone());
            let legal_status = legal.status().expect("legal settings should initialize");
            let themes = wyrmgrid_application::ThemeSettingsService::new(store.clone());
            let languages = wyrmgrid_application::LanguageSettingsService::new(store.clone());
            let display = wyrmgrid_application::DisplaySettingsService::new(store.clone());
            let onair = wyrmgrid_application::OnAirSession::with_default_store(store.clone());
            let dispatch = wyrmgrid_application::DispatchSession::with_default_provider();
            let simulator_provider =
                wyrmgrid_application::SimulatorProviderRegistration::from_manifest_json(
                    include_str!("../../../../providers/msfs2024-simconnect/provider.json"),
                    simulator_provider_path(),
                )
                .expect("bundled simulator provider manifest should validate");
            let simulator =
                wyrmgrid_application::SimulatorBridgeService::new(vec![simulator_provider]);
            let plugins = wyrmgrid_application::PluginService::new(
                app_data_directory.map(|directory| directory.join("plugins")),
                store,
                onair.clone(),
                simulator.clone(),
            );

            app.manage(DesktopState {
                startup_options: parsed_startup_options.clone(),
                onair,
                dispatch,
                plugins,
                simulator,
                legal,
                themes,
                languages,
                display,
                observability: observability::Controller::new(legal_status.telemetry_enabled),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup_options,
            platform_status,
            onair_connection_status,
            connect_onair,
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
            refresh_dispatch_weather,
            clear_dispatch_plan,
            diagnostic_log,
            clear_diagnostic_log,
            legal_status,
            acknowledge_legal,
            update_telemetry_preference,
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
            start_simulator_provider,
            stop_simulator_provider
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

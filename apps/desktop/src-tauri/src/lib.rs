mod observability;

use tauri::Manager;

struct DesktopState {
    onair: wyrmgrid_application::OnAirSession,
    dispatch: wyrmgrid_application::DispatchSession,
    plugins: wyrmgrid_application::PluginService,
    legal: wyrmgrid_application::LegalSettingsService<wyrmgrid_storage::Store>,
    themes: wyrmgrid_application::ThemeSettingsService<wyrmgrid_storage::Store>,
    observability: observability::Controller,
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
    state
        .onair
        .synchronize_company_data(trigger)
        .await
        .map_err(operation_error)
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

fn operation_error<E: Into<wyrmgrid_application::OperationError>>(
    error: E,
) -> wyrmgrid_application::OperationError {
    let operation_error = error.into();
    if operation_error.reportable {
        let report_id = observability::capture_reportable(operation_error.code);
        operation_error.with_report_id(report_id)
    } else {
        operation_error
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_directory =
                app.path().app_data_dir().ok().and_then(|directory| {
                    std::fs::create_dir_all(&directory).ok().map(|_| directory)
                });
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
            let onair = wyrmgrid_application::OnAirSession::with_default_store(store.clone());
            let dispatch = wyrmgrid_application::DispatchSession::with_default_provider();
            let plugins = wyrmgrid_application::PluginService::new(
                app_data_directory.map(|directory| directory.join("plugins")),
                store,
                onair.clone(),
            );

            app.manage(DesktopState {
                onair,
                dispatch,
                plugins,
                legal,
                themes,
                observability: observability::Controller::new(legal_status.telemetry_enabled),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            platform_status,
            onair_connection_status,
            connect_onair,
            disconnect_onair,
            synchronize_onair_company_data,
            onair_fleet_snapshot,
            onair_fbo_snapshot,
            onair_hoard_timeline,
            onair_historical_company_data,
            dispatch_status,
            import_simbrief_latest,
            refresh_dispatch_weather,
            clear_dispatch_plan,
            legal_status,
            acknowledge_legal,
            update_telemetry_preference,
            theme_status,
            select_theme,
            import_theme,
            plugin_host_status,
            approve_plugin_permissions,
            revoke_plugin_permissions,
            start_plugin,
            stop_plugin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

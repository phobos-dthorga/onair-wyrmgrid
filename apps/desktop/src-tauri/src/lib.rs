mod observability;

use tauri::Manager;

struct DesktopState {
    onair: wyrmgrid_application::OnAirSession,
    legal: wyrmgrid_application::LegalSettingsService<wyrmgrid_storage::Store>,
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
            let store = app
                .path()
                .app_data_dir()
                .ok()
                .and_then(|directory| std::fs::create_dir_all(&directory).ok().map(|_| directory))
                .and_then(|directory| {
                    wyrmgrid_storage::Store::open(directory.join("wyrmgrid.db")).ok()
                })
                .unwrap_or_else(|| {
                    wyrmgrid_storage::Store::open_in_memory()
                        .expect("in-memory Hoard fallback should initialize")
                });
            let legal = wyrmgrid_application::LegalSettingsService::new(store.clone());
            let legal_status = legal.status().expect("legal settings should initialize");

            app.manage(DesktopState {
                onair: wyrmgrid_application::OnAirSession::with_default_store(store),
                legal,
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
            legal_status,
            acknowledge_legal,
            update_telemetry_preference
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

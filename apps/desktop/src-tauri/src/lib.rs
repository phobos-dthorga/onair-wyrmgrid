use tauri::Manager;

struct DesktopState {
    onair: wyrmgrid_application::OnAirSession,
}

#[tauri::command]
fn platform_status() -> wyrmgrid_application::PlatformStatus {
    wyrmgrid_application::platform_status()
}

#[tauri::command]
fn onair_connection_status(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::ConnectionStatus, String> {
    state.onair.status().map_err(|error| error.to_string())
}

#[tauri::command]
async fn connect_onair(
    state: tauri::State<'_, DesktopState>,
    company_id: String,
    api_key: String,
) -> Result<wyrmgrid_application::ConnectionStatus, String> {
    state
        .onair
        .connect(company_id, api_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn disconnect_onair(
    state: tauri::State<'_, DesktopState>,
) -> Result<wyrmgrid_application::ConnectionStatus, String> {
    state.onair.disconnect().map_err(|error| error.to_string())
}

#[tauri::command]
async fn synchronize_onair_company_data(
    state: tauri::State<'_, DesktopState>,
    trigger: wyrmgrid_application::DataSyncTrigger,
) -> Result<wyrmgrid_application::CompanyDataSyncResult, String> {
    state
        .onair
        .synchronize_company_data(trigger)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn onair_fleet_snapshot(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::FleetSnapshotView>, String> {
    state
        .onair
        .fleet_snapshot()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn onair_fbo_snapshot(
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<wyrmgrid_application::FboSnapshotView>, String> {
    state
        .onair
        .fbo_snapshot()
        .map_err(|error| error.to_string())
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
            app.manage(DesktopState {
                onair: wyrmgrid_application::OnAirSession::with_default_store(store),
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
            onair_fbo_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

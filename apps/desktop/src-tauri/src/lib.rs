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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DesktopState {
            onair: wyrmgrid_application::OnAirSession::default(),
        })
        .invoke_handler(tauri::generate_handler![
            platform_status,
            onair_connection_status,
            connect_onair,
            disconnect_onair
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

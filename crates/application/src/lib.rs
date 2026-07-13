//! Application-level orchestration independent of Tauri and other interfaces.

use serde::Serialize;
use wyrmgrid_plugin_protocol::PLUGIN_API_VERSION;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_the_supported_plugin_api() {
        assert_eq!(platform_status().plugin_api_version, 1);
    }
}

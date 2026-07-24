use std::path::{Path, PathBuf};
use tauri::Manager;

pub const EXTENSION_DOCUMENTATION_URL: &str = "https://wyrmgr.id/";
const EXTENSION_DEVELOPER_KIT_DIRECTORY: &str = "extension-developer-kit";
const REQUIRED_EDK_FILES: [&str; 6] = [
    "package.json",
    "README.md",
    "LICENSE",
    "bin/wyrmgrid-extension.mjs",
    "schemas/schema-catalog-v1.json",
    "sdks/python/wyrmgrid_sdk/__init__.py",
];

#[derive(Debug, thiserror::Error)]
pub enum DeveloperResourceError {
    #[error("WyrmGrid could not locate its installed resources.")]
    ResourceDirectoryUnavailable,
    #[error("The bundled Extension Developer Kit is unavailable or incomplete.")]
    ExtensionDeveloperKitUnavailable,
    #[error("WyrmGrid could not open the bundled Extension Developer Kit.")]
    ExtensionDeveloperKitOpenFailed,
    #[error("WyrmGrid could not open the extension documentation.")]
    DocumentationOpenFailed,
}

impl From<DeveloperResourceError> for wyrmgrid_application::OperationError {
    fn from(error: DeveloperResourceError) -> Self {
        let (code, retryable, reportable) = match &error {
            DeveloperResourceError::ResourceDirectoryUnavailable => {
                ("developer_resources.directory_unavailable", true, true)
            }
            DeveloperResourceError::ExtensionDeveloperKitUnavailable => {
                ("developer_resources.edk_unavailable", false, true)
            }
            DeveloperResourceError::ExtensionDeveloperKitOpenFailed => {
                ("developer_resources.edk_open_failed", true, true)
            }
            DeveloperResourceError::DocumentationOpenFailed => {
                ("developer_resources.documentation_open_failed", true, false)
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

pub fn extension_developer_kit_directory(
    resource_directory: &Path,
) -> Result<PathBuf, DeveloperResourceError> {
    let directory = resource_directory.join(EXTENSION_DEVELOPER_KIT_DIRECTORY);
    if !directory.is_dir()
        || REQUIRED_EDK_FILES
            .iter()
            .any(|path| !directory.join(path).is_file())
    {
        return Err(DeveloperResourceError::ExtensionDeveloperKitUnavailable);
    }
    Ok(directory)
}

pub fn open_extension_developer_kit(app: &tauri::AppHandle) -> Result<(), DeveloperResourceError> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|_| DeveloperResourceError::ResourceDirectoryUnavailable)?;
    let directory = extension_developer_kit_directory(&resource_directory)?;
    tauri_plugin_opener::open_path(directory, None::<&str>)
        .map_err(|_| DeveloperResourceError::ExtensionDeveloperKitOpenFailed)
}

pub fn open_extension_documentation() -> Result<(), DeveloperResourceError> {
    tauri_plugin_opener::open_url(EXTENSION_DOCUMENTATION_URL, None::<&str>)
        .map_err(|_| DeveloperResourceError::DocumentationOpenFailed)
}

#[cfg(test)]
#[path = "tests/developer_resources.rs"]
mod tests;

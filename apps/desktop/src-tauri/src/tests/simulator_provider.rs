use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn temporary_workspace() -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("wyrmgrid-provider-path-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).expect("temporary workspace should be created");
    path
}

fn create_file(path: &Path) {
    fs::create_dir_all(path.parent().expect("test file should have a parent"))
        .expect("test directory should be created");
    fs::write(path, b"sanitized test executable").expect("test file should be created");
}

#[test]
fn finds_the_prepared_development_provider_when_it_is_not_beside_the_desktop() {
    let workspace = temporary_workspace();
    let provider = workspace
        .join("target/debug")
        .join(SIMULATOR_PROVIDER_EXECUTABLE);
    create_file(&provider);
    let desktop = workspace.join("another-output/wyrmgrid-desktop.exe");

    assert_eq!(
        resolve_simulator_provider_path(None, Some(desktop), &workspace, true),
        provider
    );
    fs::remove_dir_all(workspace).expect("temporary workspace should be removed");
}

#[test]
fn prefers_an_explicit_approved_path_then_an_adjacent_bundled_provider() {
    let workspace = temporary_workspace();
    let installed_directory = workspace.join("installed");
    let desktop = installed_directory.join("wyrmgrid-desktop.exe");
    let adjacent = installed_directory.join(SIMULATOR_PROVIDER_EXECUTABLE);
    create_file(&adjacent);
    let configured = workspace
        .join("maintainer-build")
        .join(SIMULATOR_PROVIDER_EXECUTABLE);

    assert_eq!(
        resolve_simulator_provider_path(
            Some(configured.clone().into_os_string()),
            Some(desktop.clone()),
            &workspace,
            false,
        ),
        configured
    );
    assert_eq!(
        resolve_simulator_provider_path(None, Some(desktop), &workspace, false),
        adjacent
    );
    fs::remove_dir_all(workspace).expect("temporary workspace should be removed");
}

#[test]
fn ignores_a_configured_path_with_the_wrong_executable_name() {
    let workspace = temporary_workspace();
    let expected = workspace
        .join("target/debug")
        .join(SIMULATOR_PROVIDER_EXECUTABLE);

    assert_eq!(
        resolve_simulator_provider_path(
            Some(workspace.join("unexpected.exe").into_os_string()),
            None,
            &workspace,
            true,
        ),
        expected
    );
    fs::remove_dir_all(workspace).expect("temporary workspace should be removed");
}

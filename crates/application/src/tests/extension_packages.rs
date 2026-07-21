use super::*;
use crate::{
    AudioProviderPackageService, OnAirSession, PluginService, SimulatorBridgeService,
    SimulatorPreferences, SimulatorSettingsService,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use tempfile::TempDir;
use wyrmgrid_storage::SimulatorPreferencesRecord;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const PLUGIN_ID: &str = "org.wyrmgrid.test.external-package";
const PROVIDER_ID: &str = "org.wyrmgrid.test.simulator-provider";
const AUDIO_PROVIDER_ID: &str = "org.wyrmgrid.test.audio-provider";

fn plugin_manifest(version: &str) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "id": PLUGIN_ID,
        "name": "External Package Test",
        "version": version,
        "api_version": 1,
        "author": "WyrmGrid tests",
        "runtime": "python",
        "entry_point": "src/main.py",
        "permissions": ["on_air_fleet_read", "map_layers_publish"]
    }))
    .unwrap()
}

fn package_bytes(
    version: &str,
    additional_entries: &[(&str, &[u8])],
    mutate_manifest: impl FnOnce(&mut serde_json::Value),
) -> Vec<u8> {
    let mut payload = BTreeMap::from([
        ("plugin.json".to_owned(), plugin_manifest(version)),
        (
            "src/main.py".to_owned(),
            b"from wyrmgrid_sdk import Plugin\n".to_vec(),
        ),
    ]);
    for (path, contents) in additional_entries {
        payload.insert((*path).to_owned(), contents.to_vec());
    }
    let files = payload
        .iter()
        .filter(|(path, _)| path.as_str() != EXTENSION_PACKAGE_MANIFEST_NAME)
        .map(|(path, contents)| {
            json!({
                "path": path,
                "size": contents.len(),
                "sha256": hex_sha256(contents),
            })
        })
        .collect::<Vec<_>>();
    let mut manifest = json!({
        "schema_version": 1,
        "kind": "ordinary_plugin",
        "id": PLUGIN_ID,
        "version": version,
        "manifest_path": "plugin.json",
        "files": files,
    });
    mutate_manifest(&mut manifest);

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(EXTENSION_PACKAGE_MANIFEST_NAME, options)
        .unwrap();
    writer
        .write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
        .unwrap();
    for (path, contents) in payload {
        writer.start_file(path, options).unwrap();
        writer.write_all(&contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn write_package(bytes: &[u8]) -> (TempDir, PathBuf) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("test.wyrmplugin");
    std::fs::write(&path, bytes).unwrap();
    (directory, path)
}

fn provider_manifest(version: &str) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema_version": 1,
        "id": PROVIDER_ID,
        "name": "Simulator Provider Test",
        "version": version,
        "bridge_protocol_version": 1,
        "author": "WyrmGrid tests",
        "entry_point": "provider.exe",
        "platforms": ["windows_x86_64"],
        "simulators": ["test_simulator"],
        "capabilities": ["telemetry_read"]
    }))
    .unwrap()
}

fn provider_package_bytes(version: &str, executable: &[u8]) -> Vec<u8> {
    provider_package_bytes_with_manifest(version, executable, |_| {})
}

fn provider_package_bytes_with_manifest(
    version: &str,
    executable: &[u8],
    mutate_provider_manifest: impl FnOnce(&mut serde_json::Value),
) -> Vec<u8> {
    let mut provider_manifest: serde_json::Value =
        serde_json::from_slice(&provider_manifest(version)).unwrap();
    mutate_provider_manifest(&mut provider_manifest);
    let payload = BTreeMap::from([
        ("provider.exe".to_owned(), executable.to_vec()),
        (
            "provider.json".to_owned(),
            serde_json::to_vec_pretty(&provider_manifest).unwrap(),
        ),
    ]);
    let files = payload
        .iter()
        .map(|(path, contents)| {
            json!({
                "path": path,
                "size": contents.len(),
                "sha256": hex_sha256(contents),
            })
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schema_version": 1,
        "kind": "simulator_provider",
        "id": PROVIDER_ID,
        "version": version,
        "manifest_path": "provider.json",
        "files": files,
    });
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(EXTENSION_PACKAGE_MANIFEST_NAME, options)
        .unwrap();
    writer
        .write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
        .unwrap();
    for (path, contents) in payload {
        writer.start_file(path, options).unwrap();
        writer.write_all(&contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn write_provider_package(bytes: &[u8]) -> (TempDir, PathBuf) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("test.wyrmprovider");
    std::fs::write(&path, bytes).unwrap();
    (directory, path)
}

fn audio_provider_package_bytes(version: &str, executable: &[u8]) -> Vec<u8> {
    let executable_name = if cfg!(windows) {
        "audio-provider.exe"
    } else {
        "audio-provider"
    };
    let platform = if cfg!(windows) {
        "windows_x86_64"
    } else if cfg!(target_os = "linux") {
        "linux_x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "macos_aarch64"
    } else {
        "macos_x86_64"
    };
    let provider_manifest = serde_json::to_vec_pretty(&json!({
        "schema_version": 2,
        "id": AUDIO_PROVIDER_ID,
        "name": "Audio Provider Test",
        "version": version,
        "audio_protocol_version": 2,
        "author": "WyrmGrid tests",
        "entry_point": "audio-provider",
        "platforms": [platform],
        "capabilities": ["source_enumeration", "pcm_s16le_capture"]
    }))
    .unwrap();
    let payload = BTreeMap::from([
        (executable_name.to_owned(), executable.to_vec()),
        ("audio-provider.json".to_owned(), provider_manifest),
    ]);
    let files = payload
        .iter()
        .map(|(path, contents)| {
            json!({
                "path": path,
                "size": contents.len(),
                "sha256": hex_sha256(contents),
            })
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schema_version": 1,
        "kind": "audio_provider",
        "id": AUDIO_PROVIDER_ID,
        "version": version,
        "manifest_path": "audio-provider.json",
        "files": files,
    });
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(EXTENSION_PACKAGE_MANIFEST_NAME, options)
        .unwrap();
    writer
        .write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
        .unwrap();
    for (path, contents) in payload {
        writer.start_file(path, options).unwrap();
        writer.write_all(&contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn write_audio_provider_package(bytes: &[u8]) -> (TempDir, PathBuf) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("test.wyrmaudio");
    std::fs::write(&path, bytes).unwrap();
    (directory, path)
}

#[test]
fn version_one_manifest_fixture_is_valid() {
    let fixture =
        include_str!("../../../../schemas/fixtures/extension-package-manifest-plugin-v1.json");
    let manifest: ExtensionPackageManifest = serde_json::from_str(fixture).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.kind.as_str(), "ordinary_plugin");
}

#[test]
fn simulator_provider_package_fixture_is_valid() {
    let fixture =
        include_str!("../../../../schemas/fixtures/simulator-provider-package-manifest-v1.json");
    let manifest: ExtensionPackageManifest = serde_json::from_str(fixture).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.kind.as_str(), "simulator_provider");
}

#[test]
fn audio_provider_package_fixture_is_valid() {
    let fixture =
        include_str!("../../../../schemas/fixtures/audio-provider-package-manifest-v1.json");
    let manifest: ExtensionPackageManifest = serde_json::from_str(fixture).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.kind.as_str(), "audio_provider");
}

#[test]
fn inspects_and_extracts_a_valid_audio_provider_package() {
    let bytes = audio_provider_package_bytes("1.2.3", b"sanitized audio provider executable");
    let (_directory, path) = write_audio_provider_package(&bytes);
    let package = read_audio_provider_package(&path).unwrap();
    let inspection = package.inspection();
    assert_eq!(inspection.id, AUDIO_PROVIDER_ID);
    assert_eq!(inspection.version, "1.2.3");
    assert_eq!(inspection.audio_protocol_version, 2);
    assert_eq!(inspection.capabilities.len(), 2);
    assert!(!inspection.publisher_verified);

    let destination = TempDir::new().unwrap();
    package.extract_to(destination.path()).unwrap();
    assert!(destination.path().join("audio-provider.json").is_file());
}

#[test]
fn inspects_and_extracts_a_valid_simulator_provider_package() {
    let bytes = provider_package_bytes("1.2.3", b"sanitized provider executable");
    let (_directory, path) = write_provider_package(&bytes);
    let package = read_simulator_provider_package(&path).unwrap();
    let inspection = package.inspection();
    assert_eq!(inspection.id, PROVIDER_ID);
    assert_eq!(inspection.version, "1.2.3");
    assert_eq!(inspection.bridge_protocol_version, 1);
    assert_eq!(inspection.simulators, vec!["test_simulator"]);
    assert!(!inspection.publisher_verified);

    let destination = TempDir::new().unwrap();
    package.extract_to(destination.path()).unwrap();
    assert_eq!(
        std::fs::read(destination.path().join("provider.json")).unwrap(),
        provider_manifest("1.2.3")
    );
    assert!(destination.path().join("provider.exe").is_file());
}

#[test]
fn rejects_a_provider_manifest_as_its_own_executable_entry_point() {
    let bytes = provider_package_bytes_with_manifest(
        "1.2.3",
        b"sanitized provider executable",
        |manifest| manifest["entry_point"] = json!("provider.json"),
    );
    let (_directory, path) = write_provider_package(&bytes);

    assert!(matches!(
        read_simulator_provider_package(&path),
        Err(ExtensionPackageError::InvalidSimulatorProviderManifest)
    ));
}

#[test]
fn deterministic_first_party_packages_pass_the_runtime_validator() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/plugin-packages");
    let expected = [
        (
            "fleet-locations.wyrmplugin",
            "org.wyrmgrid.example.fleet-locations",
        ),
        ("open-meteo.wyrmplugin", "org.wyrmgrid.provider.open-meteo"),
        (
            "aviation-weather.wyrmplugin",
            "org.wyrmgrid.provider.aviation-weather",
        ),
        ("rainviewer.wyrmplugin", "org.wyrmgrid.provider.rainviewer"),
    ];
    for (file_name, plugin_id) in expected {
        let inspection = inspect_plugin_package(&package_root.join(file_name)).unwrap();
        assert_eq!(inspection.id, plugin_id);
        assert_eq!(inspection.runtime, Some(PluginRuntime::Python));
        assert!(!inspection.publisher_verified);
    }
}

#[test]
fn deterministic_first_party_simulator_provider_passes_the_runtime_validator() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/provider-packages/msfs2024-simconnect.wyrmprovider");
    let inspection = inspect_simulator_provider_package(&package).unwrap();
    assert_eq!(
        inspection.id,
        "io.github.phobosdthorga.wyrmgrid-simconnect-msfs2024"
    );
    assert_eq!(inspection.version, "0.1.0");
    assert_eq!(inspection.bridge_protocol_version, 1);
    assert_eq!(inspection.platforms, vec![ProviderPlatform::WindowsX86_64]);
    assert_eq!(inspection.simulators, vec!["msfs_2024"]);
    assert!(!inspection.publisher_verified);
}

#[test]
fn deterministic_reference_audio_provider_passes_the_runtime_validator() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/audio-provider-packages/deterministic-fake-audio.wyrmaudio");
    let inspection = inspect_audio_provider_package(&package).unwrap();
    assert_eq!(inspection.id, "dev.wyrmgrid.fake-audio");
    assert_eq!(inspection.version, "0.3.1");
    assert_eq!(inspection.audio_protocol_version, 2);
    assert!(!inspection.publisher_verified);
}

#[test]
fn inspects_and_extracts_a_valid_plugin_package() {
    let bytes = package_bytes("1.2.3", &[], |_| {});
    let (_directory, path) = write_package(&bytes);
    let package = read_plugin_package(&path).unwrap();
    let inspection = package.inspection();
    assert_eq!(inspection.id, PLUGIN_ID);
    assert_eq!(inspection.version, "1.2.3");
    assert_eq!(inspection.runtime, Some(PluginRuntime::Python));
    assert!(!inspection.publisher_verified);
    assert_eq!(inspection.archive_sha256, hex_sha256(&bytes));

    let destination = TempDir::new().unwrap();
    package.extract_to(destination.path()).unwrap();
    assert_eq!(
        std::fs::read(destination.path().join("plugin.json")).unwrap(),
        plugin_manifest("1.2.3")
    );
    assert!(destination.path().join("src/main.py").is_file());
    assert!(
        !destination
            .path()
            .join(EXTENSION_PACKAGE_MANIFEST_NAME)
            .exists()
    );
}

#[test]
fn rejects_an_undeclared_archive_entry() {
    let bytes = package_bytes("1.0.0", &[("undeclared.txt", b"unexpected")], |manifest| {
        manifest["files"].as_array_mut().unwrap().pop();
    });
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::InvalidInventory)
    );
}

#[test]
fn rejects_a_payload_digest_mismatch() {
    let bytes = package_bytes("1.0.0", &[], |manifest| {
        manifest["files"][0]["sha256"] = json!("0".repeat(64));
    });
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::PayloadMismatch)
    );
}

#[test]
fn rejects_path_traversal_before_extraction() {
    let bytes = package_bytes("1.0.0", &[("../escape.py", b"bad")], |_| {});
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::UnsafeArchiveEntry)
    );
}

#[test]
fn rejects_case_colliding_paths() {
    let bytes = package_bytes("1.0.0", &[("SRC/main.py", b"collision")], |_| {});
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::UnsafeArchiveEntry)
    );
}

#[test]
fn rejects_a_package_and_plugin_identity_mismatch() {
    let bytes = package_bytes("1.0.0", &[], |manifest| {
        manifest["id"] = json!("org.wyrmgrid.test.different");
    });
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::InvalidPluginManifest)
    );
}

#[test]
fn rejects_an_unsupported_package_schema() {
    let bytes = package_bytes("1.0.0", &[], |manifest| {
        manifest["schema_version"] = json!(2);
    });
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::UnsupportedSchema)
    );
}

#[test]
fn rejects_a_symlink_archive_entry() {
    let payload = plugin_manifest("1.0.0");
    let files = json!([
        {
            "path": "plugin.json",
            "size": payload.len(),
            "sha256": hex_sha256(&payload),
        },
        {
            "path": "src/main.py",
            "size": 6,
            "sha256": hex_sha256(b"target"),
        }
    ]);
    let manifest = json!({
        "schema_version": 1,
        "kind": "ordinary_plugin",
        "id": PLUGIN_ID,
        "version": "1.0.0",
        "manifest_path": "plugin.json",
        "files": files,
    });
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer
        .start_file(
            EXTENSION_PACKAGE_MANIFEST_NAME,
            SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    writer
        .start_file("plugin.json", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&payload).unwrap();
    writer
        .add_symlink(
            "src/main.py",
            "target",
            SimpleFileOptions::default().unix_permissions(0o777),
        )
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let (_directory, path) = write_package(&bytes);
    assert_eq!(
        inspect_plugin_package(&path),
        Err(ExtensionPackageError::UnsafeArchiveEntry)
    );
}

#[test]
fn installs_disables_updates_rolls_back_and_removes_a_managed_plugin() {
    let package_root = TempDir::new().unwrap();
    let store = Store::open_in_memory().unwrap();
    let service =
        ExtensionPackageService::new(Some(package_root.path().join("extensions-v1")), store);
    assert!(service.available());

    let (version_one_directory, version_one_path) =
        write_package(&package_bytes("1.0.0", &[], |_| {}));
    let installed = service.install_plugin_package(&version_one_path).unwrap();
    assert_eq!(installed.active_version, "1.0.0");
    assert_eq!(installed.rollback_version, None);
    assert!(installed.enabled);
    let active_root = service.active_plugin_roots().unwrap().remove(0);
    assert!(active_root.join("plugin.json").is_file());

    let disabled = service.set_plugin_enabled(PLUGIN_ID, false).unwrap();
    assert!(!disabled.enabled);
    assert!(service.active_plugin_roots().unwrap().is_empty());

    let (version_two_directory, version_two_path) =
        write_package(&package_bytes("1.1.0", &[], |_| {}));
    let updated = service.install_plugin_package(&version_two_path).unwrap();
    assert_eq!(updated.active_version, "1.1.0");
    assert_eq!(updated.rollback_version.as_deref(), Some("1.0.0"));
    assert!(!updated.enabled);
    assert_eq!(updated.installed_versions.len(), 2);

    let rolled_back = service.rollback_plugin(PLUGIN_ID).unwrap();
    assert_eq!(rolled_back.active_version, "1.0.0");
    assert_eq!(rolled_back.rollback_version.as_deref(), Some("1.1.0"));
    service.set_plugin_enabled(PLUGIN_ID, true).unwrap();
    assert_eq!(service.active_plugin_roots().unwrap().len(), 1);

    service.remove_plugin(PLUGIN_ID).unwrap();
    assert!(service.list_plugin_packages().unwrap().is_empty());
    assert!(service.active_plugin_roots().unwrap().is_empty());
    drop((version_one_directory, version_two_directory));
}

#[test]
fn managed_package_operations_fail_closed_when_the_root_is_replaced() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("extensions-v1");
    let service =
        ExtensionPackageService::new(Some(root.clone()), Store::open_in_memory().unwrap());
    assert!(service.available());

    std::fs::remove_dir_all(&root).unwrap();
    std::fs::write(&root, b"redirected package root").unwrap();

    assert!(matches!(
        service.list_simulator_provider_packages(),
        Err(ExtensionPackageManagementError::RootUnavailable)
    ));
}

#[test]
fn installs_disables_updates_rolls_back_and_removes_a_simulator_provider() {
    let package_root = TempDir::new().unwrap();
    let service = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        Store::open_in_memory().unwrap(),
    );
    let (one_directory, one_path) =
        write_provider_package(&provider_package_bytes("1.0.0", b"provider one"));
    let installed = service
        .install_simulator_provider_package(&one_path)
        .unwrap();
    assert_eq!(installed.active_version, "1.0.0");
    assert!(installed.enabled);
    assert_eq!(
        service.active_managed_simulator_providers().unwrap()[0]
            .manifest
            .id,
        PROVIDER_ID
    );

    let disabled = service
        .set_simulator_provider_enabled(PROVIDER_ID, false)
        .unwrap();
    assert!(!disabled.enabled);
    assert!(
        service
            .active_managed_simulator_providers()
            .unwrap()
            .is_empty()
    );

    let (two_directory, two_path) =
        write_provider_package(&provider_package_bytes("2.0.0", b"provider two"));
    let updated = service
        .install_simulator_provider_package(&two_path)
        .unwrap();
    assert_eq!(updated.active_version, "2.0.0");
    assert_eq!(updated.rollback_version.as_deref(), Some("1.0.0"));
    assert!(!updated.enabled);

    let rolled_back = service.rollback_simulator_provider(PROVIDER_ID).unwrap();
    assert_eq!(rolled_back.active_version, "1.0.0");
    service.remove_simulator_provider(PROVIDER_ID).unwrap();
    assert!(
        service
            .list_simulator_provider_packages()
            .unwrap()
            .is_empty()
    );
    drop((one_directory, two_directory));
}

#[test]
fn manages_audio_provider_packages_and_clears_disabled_selection() {
    let package_root = TempDir::new().unwrap();
    let store = Store::open_in_memory().unwrap();
    let packages = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        store.clone(),
    );
    let service = AudioProviderPackageService::new(packages, store);
    let (one_directory, one_path) = write_audio_provider_package(&audio_provider_package_bytes(
        "1.0.0",
        b"audio provider one",
    ));
    let installed = service.install_package(&one_path).unwrap();
    assert!(installed.enabled);
    assert_eq!(service.selected_provider_id().unwrap(), None);

    service.select_provider(AUDIO_PROVIDER_ID).unwrap();
    assert_eq!(
        service.selected_provider_id().unwrap().as_deref(),
        Some(AUDIO_PROVIDER_ID)
    );
    assert_eq!(
        service.provider().unwrap().unwrap().provider_id(),
        AUDIO_PROVIDER_ID
    );

    let disabled = service.set_enabled(AUDIO_PROVIDER_ID, false).unwrap();
    assert!(!disabled.enabled);
    assert_eq!(service.selected_provider_id().unwrap(), None);
    assert!(service.provider().unwrap().is_none());

    service.set_enabled(AUDIO_PROVIDER_ID, true).unwrap();
    let (two_directory, two_path) = write_audio_provider_package(&audio_provider_package_bytes(
        "2.0.0",
        b"audio provider two",
    ));
    let updated = service.install_package(&two_path).unwrap();
    assert_eq!(updated.active_version, "2.0.0");
    assert_eq!(updated.rollback_version.as_deref(), Some("1.0.0"));
    let rolled_back = service.rollback(AUDIO_PROVIDER_ID).unwrap();
    assert_eq!(rolled_back.active_version, "1.0.0");
    service.remove(AUDIO_PROVIDER_ID).unwrap();
    assert!(service.list_packages().unwrap().is_empty());
    drop((one_directory, two_directory));
}

#[test]
fn simulator_supervisor_tracks_enabled_managed_provider_packages() {
    let package_root = TempDir::new().unwrap();
    let packages = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        Store::open_in_memory().unwrap(),
    );
    let (provider_directory, provider_path) =
        write_provider_package(&provider_package_bytes("1.0.0", b"provider executable"));
    packages
        .install_simulator_provider_package(&provider_path)
        .unwrap();
    let bridge = SimulatorBridgeService::with_extension_packages(Vec::new(), packages, None);

    assert_eq!(bridge.provider_ids(), vec![PROVIDER_ID]);
    assert_eq!(bridge.status().unwrap().providers[0].id, PROVIDER_ID);
    bridge
        .set_managed_provider_enabled(PROVIDER_ID, false)
        .unwrap();
    assert!(bridge.provider_ids().is_empty());
    assert!(bridge.status().unwrap().providers.is_empty());
    drop(provider_directory);
}

#[test]
fn removing_a_selected_provider_clears_its_saved_auto_start_state() {
    let package_root = TempDir::new().unwrap();
    let store = Store::open_in_memory().unwrap();
    let packages = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        store.clone(),
    );
    let (provider_directory, provider_path) =
        write_provider_package(&provider_package_bytes("1.0.0", b"provider executable"));
    packages
        .install_simulator_provider_package(&provider_path)
        .unwrap();
    let bridge = SimulatorBridgeService::with_extension_packages(Vec::new(), packages, None);
    let settings = SimulatorSettingsService::with_bridge(store.clone(), bridge);
    settings
        .update(SimulatorPreferences {
            selected_provider_id: Some(PROVIDER_ID.to_owned()),
            start_with_wyrmgrid: true,
        })
        .unwrap();

    settings.remove_managed_provider(PROVIDER_ID).unwrap();

    assert_eq!(
        store.load_simulator_preferences_record().unwrap(),
        Some(SimulatorPreferencesRecord {
            selected_provider_id: None,
            start_with_wyrmgrid: false,
        })
    );
    drop(provider_directory);
}

#[test]
fn disabling_a_selected_provider_revokes_saved_auto_start() {
    let package_root = TempDir::new().unwrap();
    let store = Store::open_in_memory().unwrap();
    let packages = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        store.clone(),
    );
    let (provider_directory, provider_path) =
        write_provider_package(&provider_package_bytes("1.0.0", b"provider executable"));
    packages
        .install_simulator_provider_package(&provider_path)
        .unwrap();
    let bridge = SimulatorBridgeService::with_extension_packages(Vec::new(), packages, None);
    let settings = SimulatorSettingsService::with_bridge(store.clone(), bridge);
    settings
        .update(SimulatorPreferences {
            selected_provider_id: Some(PROVIDER_ID.to_owned()),
            start_with_wyrmgrid: true,
        })
        .unwrap();

    let disabled = settings
        .set_managed_provider_enabled(PROVIDER_ID, false)
        .unwrap();

    assert!(!disabled.enabled);
    assert_eq!(
        store.load_simulator_preferences_record().unwrap(),
        Some(SimulatorPreferencesRecord {
            selected_provider_id: Some(PROVIDER_ID.to_owned()),
            start_with_wyrmgrid: false,
        })
    );
    drop(provider_directory);
}

#[test]
fn rejects_reusing_a_plugin_version_for_different_package_contents() {
    let package_root = TempDir::new().unwrap();
    let service = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        Store::open_in_memory().unwrap(),
    );
    let (first_directory, first_path) = write_package(&package_bytes("1.0.0", &[], |_| {}));
    service.install_plugin_package(&first_path).unwrap();
    let (changed_directory, changed_path) = write_package(&package_bytes(
        "1.0.0",
        &[("NOTICE.txt", b"changed package")],
        |_| {},
    ));
    assert!(matches!(
        service.install_plugin_package(&changed_path),
        Err(ExtensionPackageManagementError::VersionConflict)
    ));
    drop((first_directory, changed_directory));
}

#[test]
fn first_party_seeding_updates_only_an_existing_first_party_active_version() {
    let package_root = TempDir::new().unwrap();
    let service = ExtensionPackageService::new(
        Some(package_root.path().join("extensions-v1")),
        Store::open_in_memory().unwrap(),
    );
    let (one_directory, one_path) = write_package(&package_bytes("1.0.0", &[], |_| {}));
    service.seed_first_party_plugin_package(&one_path).unwrap();
    let (two_directory, two_path) = write_package(&package_bytes("2.0.0", &[], |_| {}));
    let updated = service.seed_first_party_plugin_package(&two_path).unwrap();
    assert_eq!(updated.active_version, "2.0.0");
    assert_eq!(updated.rollback_version.as_deref(), Some("1.0.0"));

    let (three_directory, three_path) = write_package(&package_bytes("3.0.0", &[], |_| {}));
    service.install_plugin_package(&three_path).unwrap();
    let (four_directory, four_path) = write_package(&package_bytes("4.0.0", &[], |_| {}));
    let protected = service.seed_first_party_plugin_package(&four_path).unwrap();
    assert_eq!(protected.active_version, "3.0.0");
    assert_eq!(protected.rollback_version.as_deref(), Some("2.0.0"));
    assert_eq!(protected.installed_versions.len(), 4);
    drop((
        one_directory,
        two_directory,
        three_directory,
        four_directory,
    ));
}

#[test]
fn plugin_supervisor_discovers_only_enabled_managed_package_payloads() {
    let application_data = TempDir::new().unwrap();
    let store = Store::open_in_memory().unwrap();
    let service = PluginService::new(
        Some(application_data.path().join("plugins")),
        store.clone(),
        OnAirSession::with_default_store(store),
        SimulatorBridgeService::new(Vec::new()),
    );
    let (package_directory, package_path) = write_package(&package_bytes("1.0.0", &[], |_| {}));
    service.install_plugin_package(&package_path).unwrap();
    assert!(
        service
            .status()
            .unwrap()
            .plugins
            .iter()
            .any(|plugin| plugin.id == PLUGIN_ID && plugin.version == "1.0.0")
    );

    service
        .set_managed_plugin_enabled(PLUGIN_ID, false)
        .unwrap();
    assert!(
        service
            .status()
            .unwrap()
            .plugins
            .iter()
            .all(|plugin| plugin.id != PLUGIN_ID)
    );
    drop(package_directory);
}

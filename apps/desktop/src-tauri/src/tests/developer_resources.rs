use super::{
    DeveloperResourceError, EXTENSION_DOCUMENTATION_URL, extension_developer_kit_directory,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wyrmgrid-edk-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn accepts_only_a_complete_bundled_edk_directory() {
    let resources = TestDirectory::new();
    let edk = resources.path().join("extension-developer-kit");
    for path in [
        "package.json",
        "README.md",
        "LICENSE",
        "bin/wyrmgrid-extension.mjs",
        "schemas/schema-catalog-v1.json",
        "sdks/python/wyrmgrid_sdk/__init__.py",
    ] {
        let destination = edk.join(path);
        fs::create_dir_all(destination.parent().expect("file should have a parent"))
            .expect("parent should be created");
        fs::write(destination, b"fixture").expect("fixture should be written");
    }

    assert_eq!(
        extension_developer_kit_directory(resources.path()).unwrap(),
        edk
    );
}

#[test]
fn rejects_a_missing_or_partial_bundled_edk_directory() {
    let resources = TestDirectory::new();
    fs::create_dir_all(resources.path().join("extension-developer-kit"))
        .expect("partial directory should be created");

    assert!(matches!(
        extension_developer_kit_directory(resources.path()),
        Err(DeveloperResourceError::ExtensionDeveloperKitUnavailable)
    ));
}

#[test]
fn documentation_action_is_pinned_to_the_public_wyrmgrid_site() {
    assert_eq!(EXTENSION_DOCUMENTATION_URL, "https://wyrmgr.id/");
}

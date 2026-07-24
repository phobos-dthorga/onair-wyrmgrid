use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_audio_codec_protocol::{
    AudioCodecCapability, AudioCodecManifest, AudioCodecPlatform, AudioCodecProfile,
};
use wyrmgrid_audio_provider_protocol::{
    AudioProviderCapability, AudioProviderManifest, AudioProviderPlatform,
};
use wyrmgrid_bridge_protocol::{BridgeCapability, ProviderManifest, ProviderPlatform};
use wyrmgrid_plugin_protocol::{PluginManifest, PluginRuntime};
use wyrmgrid_storage::{NewExtensionPackageVersionRecord, StorageError, Store};
use zip::{CompressionMethod, ZipArchive};

use crate::{
    codec_executable_in, current_audio_provider_platform, current_codec_platform,
    provider_executable_in,
};

pub const EXTENSION_PACKAGE_SCHEMA_VERSION: u32 = 1;
pub const EXTENSION_PACKAGE_MANIFEST_NAME: &str = "wyrmgrid-package.json";
pub const PLUGIN_PACKAGE_EXTENSION: &str = "wyrmplugin";
pub const PLUGIN_PACKAGE_MEDIA_TYPE: &str = "application/vnd.wyrmgrid.plugin-package+zip";
pub const SIMULATOR_PROVIDER_PACKAGE_EXTENSION: &str = "wyrmprovider";
pub const SIMULATOR_PROVIDER_PACKAGE_MEDIA_TYPE: &str =
    "application/vnd.wyrmgrid.simulator-provider-package+zip";
pub const AUDIO_PROVIDER_PACKAGE_EXTENSION: &str = "wyrmaudio";
pub const AUDIO_PROVIDER_PACKAGE_MEDIA_TYPE: &str =
    "application/vnd.wyrmgrid.audio-provider-package+zip";
pub const AUDIO_CODEC_PACKAGE_EXTENSION: &str = "wyrmcodec";
pub const AUDIO_CODEC_PACKAGE_MEDIA_TYPE: &str = "application/vnd.wyrmgrid.audio-codec-package+zip";
pub const EXTENSION_PACKAGE_SOURCE_LOCAL_FILE: &str = "local_file";
pub const EXTENSION_PACKAGE_SOURCE_FIRST_PARTY: &str = "first_party";

const STAGING_DIRECTORY: &str = ".staging";
const REMOVAL_DIRECTORY: &str = ".removing";

const MAX_PACKAGE_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PACKAGE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACKAGE_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_PACKAGE_FILES: usize = 512;
const MAX_PACKAGE_PATH_BYTES: usize = 240;
const MAX_PACKAGE_PATH_DEPTH: usize = 8;
const MAX_PACKAGE_COMPONENT_BYTES: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPackageKind {
    OrdinaryPlugin,
    SimulatorProvider,
    AudioProvider,
    AudioCodecProvider,
}

impl ExtensionPackageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OrdinaryPlugin => "ordinary_plugin",
            Self::SimulatorProvider => "simulator_provider",
            Self::AudioProvider => "audio_provider",
            Self::AudioCodecProvider => "audio_codec_provider",
        }
    }

    const fn manifest_path(self) -> &'static str {
        match self {
            Self::OrdinaryPlugin => "plugin.json",
            Self::SimulatorProvider => "provider.json",
            Self::AudioProvider => "audio-provider.json",
            Self::AudioCodecProvider => "audio-codec.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPackageFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionPackageManifest {
    pub schema_version: u32,
    pub kind: ExtensionPackageKind,
    pub id: String,
    pub version: String,
    pub manifest_path: String,
    pub files: Vec<ExtensionPackageFile>,
}

impl ExtensionPackageManifest {
    pub fn validate(&self) -> Result<(), ExtensionPackageError> {
        if self.schema_version != EXTENSION_PACKAGE_SCHEMA_VERSION {
            return Err(ExtensionPackageError::UnsupportedSchema);
        }
        if !valid_reverse_domain_id(&self.id) || !valid_semantic_version(&self.version) {
            return Err(ExtensionPackageError::InvalidIdentity);
        }
        if self.manifest_path != self.kind.manifest_path() {
            return Err(ExtensionPackageError::UnsupportedPackageKind);
        }
        if self.files.is_empty() || self.files.len() > MAX_PACKAGE_FILES {
            return Err(ExtensionPackageError::InvalidInventory);
        }

        let mut paths = BTreeSet::new();
        let mut case_folded_paths = BTreeSet::new();
        let mut expanded_bytes = 0_u64;
        for file in &self.files {
            if !valid_package_path(&file.path)
                || file.path == EXTENSION_PACKAGE_MANIFEST_NAME
                || file.size == 0
                || file.size > MAX_PACKAGE_FILE_BYTES
                || !valid_sha256(&file.sha256)
                || !paths.insert(file.path.clone())
                || !case_folded_paths.insert(file.path.to_ascii_lowercase())
            {
                return Err(ExtensionPackageError::InvalidInventory);
            }
            expanded_bytes = expanded_bytes
                .checked_add(file.size)
                .ok_or(ExtensionPackageError::InvalidInventory)?;
        }
        if expanded_bytes > MAX_PACKAGE_EXPANDED_BYTES || !paths.contains(&self.manifest_path) {
            return Err(ExtensionPackageError::InvalidInventory);
        }
        Ok(())
    }

    pub fn expanded_size(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginPackageInspection {
    pub package_schema_version: u32,
    pub package_kind: ExtensionPackageKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<PluginRuntime>,
    pub archive_sha256: String,
    pub archive_size: u64,
    pub expanded_size: u64,
    pub file_count: usize,
    pub publisher_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedPluginPackageView {
    pub id: String,
    pub name: String,
    pub author: String,
    pub active_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_version: Option<String>,
    pub enabled: bool,
    pub installed_versions: Vec<String>,
    pub active_archive_sha256: String,
    pub source: String,
    pub publisher_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulatorProviderPackageInspection {
    pub package_schema_version: u32,
    pub package_kind: ExtensionPackageKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub bridge_protocol_version: u32,
    pub platforms: Vec<ProviderPlatform>,
    pub simulators: Vec<String>,
    pub capabilities: Vec<BridgeCapability>,
    pub archive_sha256: String,
    pub archive_size: u64,
    pub expanded_size: u64,
    pub file_count: usize,
    pub publisher_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedSimulatorProviderPackageView {
    pub id: String,
    pub name: String,
    pub author: String,
    pub active_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_version: Option<String>,
    pub enabled: bool,
    pub installed_versions: Vec<String>,
    pub active_archive_sha256: String,
    pub source: String,
    pub publisher_verified: bool,
    pub bridge_protocol_version: u32,
    pub platforms: Vec<ProviderPlatform>,
    pub simulators: Vec<String>,
    pub capabilities: Vec<BridgeCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioProviderPackageInspection {
    pub package_schema_version: u32,
    pub package_kind: ExtensionPackageKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub audio_protocol_version: u32,
    pub platforms: Vec<AudioProviderPlatform>,
    pub capabilities: Vec<AudioProviderCapability>,
    pub archive_sha256: String,
    pub archive_size: u64,
    pub expanded_size: u64,
    pub file_count: usize,
    pub publisher_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedAudioProviderPackageView {
    pub id: String,
    pub name: String,
    pub author: String,
    pub active_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_version: Option<String>,
    pub enabled: bool,
    pub installed_versions: Vec<String>,
    pub active_archive_sha256: String,
    pub source: String,
    pub publisher_verified: bool,
    pub audio_protocol_version: u32,
    pub platforms: Vec<AudioProviderPlatform>,
    pub capabilities: Vec<AudioProviderCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioCodecPackageInspection {
    pub package_schema_version: u32,
    pub package_kind: ExtensionPackageKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub codec_protocol_version: u32,
    pub platforms: Vec<AudioCodecPlatform>,
    pub capabilities: Vec<AudioCodecCapability>,
    pub profiles: Vec<AudioCodecProfile>,
    pub archive_sha256: String,
    pub archive_size: u64,
    pub expanded_size: u64,
    pub file_count: usize,
    pub publisher_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedAudioCodecPackageView {
    pub id: String,
    pub name: String,
    pub author: String,
    pub active_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_version: Option<String>,
    pub enabled: bool,
    pub installed_versions: Vec<String>,
    pub active_archive_sha256: String,
    pub source: String,
    pub publisher_verified: bool,
    pub codec_protocol_version: u32,
    pub platforms: Vec<AudioCodecPlatform>,
    pub capabilities: Vec<AudioCodecCapability>,
    pub profiles: Vec<AudioCodecProfile>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionPackageError {
    #[error("The selected extension package could not be read.")]
    Unavailable,
    #[error("The selected extension package exceeds the 32 MiB archive limit.")]
    ArchiveTooLarge,
    #[error("That file is not a valid WyrmGrid extension package.")]
    InvalidArchive,
    #[error("That extension package uses an unsupported package schema.")]
    UnsupportedSchema,
    #[error("That extension package kind or manifest location is not supported.")]
    UnsupportedPackageKind,
    #[error("That extension package identity or version is invalid.")]
    InvalidIdentity,
    #[error("That extension package has an invalid or unsafe file inventory.")]
    InvalidInventory,
    #[error("That extension package contains an unsafe archive entry.")]
    UnsafeArchiveEntry,
    #[error("That extension package payload does not match its declared inventory.")]
    PayloadMismatch,
    #[error("That extension package does not contain a valid plugin manifest.")]
    InvalidPluginManifest,
    #[error("That extension package does not contain a valid simulator provider manifest.")]
    InvalidSimulatorProviderManifest,
    #[error("That extension package does not contain a valid audio provider manifest.")]
    InvalidAudioProviderManifest,
    #[error("That extension package does not contain a valid audio codec manifest.")]
    InvalidAudioCodecManifest,
    #[error("WyrmGrid could not stage the extension package.")]
    ExtractionFailed,
}

#[derive(Debug, Error)]
pub enum ExtensionPackageManagementError {
    #[error(transparent)]
    InvalidPackage(#[from] ExtensionPackageError),
    #[error("Local extension-package storage is unavailable.")]
    RootUnavailable,
    #[error("WyrmGrid could not read or save extension-package state.")]
    StorageUnavailable,
    #[error("That extension version is already installed from different package contents.")]
    VersionConflict,
    #[error("No previous extension version is available for rollback.")]
    RollbackUnavailable,
    #[error("The extension-package manager is unavailable.")]
    StateUnavailable,
    #[error("WyrmGrid could not complete the extension-package file operation.")]
    FileOperation,
}

#[derive(Clone)]
pub struct ExtensionPackageService {
    inner: Arc<ExtensionPackageServiceInner>,
}

struct ExtensionPackageServiceInner {
    root: Option<PathBuf>,
    store: Store,
    mutation_lock: Mutex<()>,
}

pub(crate) struct ActiveManagedPlugin {
    pub id: String,
    pub version: String,
    pub root: PathBuf,
}

pub(crate) struct ActiveManagedSimulatorProvider {
    pub manifest: ProviderManifest,
    pub root: PathBuf,
}

pub(crate) struct ActiveManagedAudioProvider {
    pub manifest: AudioProviderManifest,
    pub root: PathBuf,
}

pub(crate) struct ActiveManagedAudioCodec {
    pub manifest: AudioCodecManifest,
    pub root: PathBuf,
}

impl ExtensionPackageService {
    pub fn new(root: Option<PathBuf>, store: Store) -> Self {
        let root = root
            .and_then(|root| prepare_package_root(root).ok())
            .filter(|root| recover_pending_removals(root, &store).is_ok());
        Self {
            inner: Arc::new(ExtensionPackageServiceInner {
                root,
                store,
                mutation_lock: Mutex::new(()),
            }),
        }
    }

    pub fn available(&self) -> bool {
        self.inner.root.is_some()
    }

    pub fn install_plugin_package(
        &self,
        path: &Path,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let package = read_plugin_package(path)?;
        self.install_validated_plugin_package(package, EXTENSION_PACKAGE_SOURCE_LOCAL_FILE, true)
    }

    pub fn seed_first_party_plugin_package(
        &self,
        path: &Path,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let package = read_plugin_package(path)?;
        self.install_validated_plugin_package(package, EXTENSION_PACKAGE_SOURCE_FIRST_PARTY, false)
    }

    pub fn install_simulator_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let package = read_simulator_provider_package(path)?;
        self.install_validated_simulator_provider_package(
            package,
            EXTENSION_PACKAGE_SOURCE_LOCAL_FILE,
            true,
        )
    }

    pub fn seed_first_party_simulator_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let package = read_simulator_provider_package(path)?;
        self.install_validated_simulator_provider_package(
            package,
            EXTENSION_PACKAGE_SOURCE_FIRST_PARTY,
            false,
        )
    }

    pub fn install_audio_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let package = read_audio_provider_package(path)?;
        self.install_validated_audio_provider_package(
            package,
            EXTENSION_PACKAGE_SOURCE_LOCAL_FILE,
            true,
        )
    }

    pub fn seed_first_party_audio_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let package = read_audio_provider_package(path)?;
        self.install_validated_audio_provider_package(
            package,
            EXTENSION_PACKAGE_SOURCE_FIRST_PARTY,
            false,
        )
    }

    pub fn install_audio_codec_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let package = read_audio_codec_package(path)?;
        self.install_validated_audio_codec_package(
            package,
            EXTENSION_PACKAGE_SOURCE_LOCAL_FILE,
            true,
        )
    }

    pub fn seed_first_party_audio_codec_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let package = read_audio_codec_package(path)?;
        self.install_validated_audio_codec_package(
            package,
            EXTENSION_PACKAGE_SOURCE_FIRST_PARTY,
            false,
        )
    }

    pub fn list_plugin_packages(
        &self,
    ) -> Result<Vec<ManagedPluginPackageView>, ExtensionPackageManagementError> {
        let root = self.root()?;
        let states = self
            .inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::OrdinaryPlugin.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        states
            .into_iter()
            .map(|state| managed_plugin_view(&self.inner.store, root, state))
            .collect()
    }

    pub fn list_simulator_provider_packages(
        &self,
    ) -> Result<Vec<ManagedSimulatorProviderPackageView>, ExtensionPackageManagementError> {
        let root = self.root()?;
        let states = self
            .inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::SimulatorProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        states
            .into_iter()
            .map(|state| managed_simulator_provider_view(&self.inner.store, root, state))
            .collect()
    }

    pub fn list_audio_provider_packages(
        &self,
    ) -> Result<Vec<ManagedAudioProviderPackageView>, ExtensionPackageManagementError> {
        let root = self.root()?;
        let states = self
            .inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::AudioProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        states
            .into_iter()
            .map(|state| managed_audio_provider_view(&self.inner.store, root, state))
            .collect()
    }

    pub fn list_audio_codec_packages(
        &self,
    ) -> Result<Vec<ManagedAudioCodecPackageView>, ExtensionPackageManagementError> {
        let root = self.root()?;
        let states = self
            .inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::AudioCodecProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        states
            .into_iter()
            .map(|state| managed_audio_codec_view(&self.inner.store, root, state))
            .collect()
    }

    pub fn active_plugin_roots(&self) -> Result<Vec<PathBuf>, ExtensionPackageManagementError> {
        self.active_managed_plugins()
            .map(|plugins| plugins.into_iter().map(|plugin| plugin.root).collect())
    }

    pub(crate) fn active_managed_plugins(
        &self,
    ) -> Result<Vec<ActiveManagedPlugin>, ExtensionPackageManagementError> {
        let root = self.root()?;
        self.inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::OrdinaryPlugin.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .into_iter()
            .filter(|state| state.enabled)
            .map(|state| {
                let version = self
                    .inner
                    .store
                    .load_extension_package_version_record(
                        &state.package_kind,
                        &state.extension_id,
                        &state.active_version,
                    )
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
                    .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
                Ok(ActiveManagedPlugin {
                    root: managed_payload_root(
                        root,
                        ExtensionPackageKind::OrdinaryPlugin,
                        &state.extension_id,
                        &state.active_version,
                        &version.archive_sha256,
                    )?,
                    id: state.extension_id,
                    version: state.active_version,
                })
            })
            .collect()
    }

    pub(crate) fn active_managed_simulator_providers(
        &self,
    ) -> Result<Vec<ActiveManagedSimulatorProvider>, ExtensionPackageManagementError> {
        let root = self.root()?;
        self.inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::SimulatorProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .into_iter()
            .filter(|state| state.enabled)
            .map(|state| {
                let version = self
                    .inner
                    .store
                    .load_extension_package_version_record(
                        &state.package_kind,
                        &state.extension_id,
                        &state.active_version,
                    )
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
                    .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
                let manifest: ProviderManifest =
                    serde_json::from_str(&version.extension_manifest_json)
                        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                manifest
                    .validate()
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                if manifest.id != state.extension_id || manifest.version != state.active_version {
                    return Err(ExtensionPackageManagementError::StorageUnavailable);
                }
                Ok(ActiveManagedSimulatorProvider {
                    root: managed_payload_root(
                        root,
                        ExtensionPackageKind::SimulatorProvider,
                        &state.extension_id,
                        &state.active_version,
                        &version.archive_sha256,
                    )?,
                    manifest,
                })
            })
            .collect()
    }

    pub(crate) fn active_managed_audio_providers(
        &self,
    ) -> Result<Vec<ActiveManagedAudioProvider>, ExtensionPackageManagementError> {
        let root = self.root()?;
        self.inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::AudioProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .into_iter()
            .filter(|state| state.enabled)
            .map(|state| {
                let version = self
                    .inner
                    .store
                    .load_extension_package_version_record(
                        &state.package_kind,
                        &state.extension_id,
                        &state.active_version,
                    )
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
                    .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
                let manifest: AudioProviderManifest =
                    serde_json::from_str(&version.extension_manifest_json)
                        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                manifest
                    .validate()
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                if manifest.id != state.extension_id || manifest.version != state.active_version {
                    return Err(ExtensionPackageManagementError::StorageUnavailable);
                }
                Ok(ActiveManagedAudioProvider {
                    root: managed_payload_root(
                        root,
                        ExtensionPackageKind::AudioProvider,
                        &state.extension_id,
                        &state.active_version,
                        &version.archive_sha256,
                    )?,
                    manifest,
                })
            })
            .collect()
    }

    pub(crate) fn active_managed_audio_codecs(
        &self,
    ) -> Result<Vec<ActiveManagedAudioCodec>, ExtensionPackageManagementError> {
        let root = self.root()?;
        self.inner
            .store
            .list_extension_package_state_records(ExtensionPackageKind::AudioCodecProvider.as_str())
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .into_iter()
            .filter(|state| state.enabled)
            .map(|state| {
                let version = self
                    .inner
                    .store
                    .load_extension_package_version_record(
                        &state.package_kind,
                        &state.extension_id,
                        &state.active_version,
                    )
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
                    .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
                let manifest: AudioCodecManifest =
                    serde_json::from_str(&version.extension_manifest_json)
                        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                manifest
                    .validate()
                    .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
                if manifest.id != state.extension_id || manifest.version != state.active_version {
                    return Err(ExtensionPackageManagementError::StorageUnavailable);
                }
                Ok(ActiveManagedAudioCodec {
                    root: managed_payload_root(
                        root,
                        ExtensionPackageKind::AudioCodecProvider,
                        &state.extension_id,
                        &state.active_version,
                        &version.archive_sha256,
                    )?,
                    manifest,
                })
            })
            .collect()
    }

    pub fn set_plugin_enabled(
        &self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .set_extension_package_enabled(
                ExtensionPackageKind::OrdinaryPlugin.as_str(),
                plugin_id,
                enabled,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        self.load_plugin_view(plugin_id)
    }

    pub fn set_simulator_provider_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .set_extension_package_enabled(
                ExtensionPackageKind::SimulatorProvider.as_str(),
                provider_id,
                enabled,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        self.load_simulator_provider_view(provider_id)
    }

    pub fn set_audio_provider_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .set_extension_package_enabled(
                ExtensionPackageKind::AudioProvider.as_str(),
                provider_id,
                enabled,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        self.load_audio_provider_view(provider_id)
    }

    pub fn set_audio_codec_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .set_extension_package_enabled(
                ExtensionPackageKind::AudioCodecProvider.as_str(),
                provider_id,
                enabled,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
        self.load_audio_codec_view(provider_id)
    }

    pub fn rollback_plugin(
        &self,
        plugin_id: &str,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .rollback_extension_package(ExtensionPackageKind::OrdinaryPlugin.as_str(), plugin_id)
            .map_err(|error| match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::RollbackUnavailable,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            })?;
        self.load_plugin_view(plugin_id)
    }

    pub fn rollback_simulator_provider(
        &self,
        provider_id: &str,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .rollback_extension_package(
                ExtensionPackageKind::SimulatorProvider.as_str(),
                provider_id,
            )
            .map_err(|error| match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::RollbackUnavailable,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            })?;
        self.load_simulator_provider_view(provider_id)
    }

    pub fn rollback_audio_provider(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .rollback_extension_package(ExtensionPackageKind::AudioProvider.as_str(), provider_id)
            .map_err(|error| match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::RollbackUnavailable,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            })?;
        self.load_audio_provider_view(provider_id)
    }

    pub fn rollback_audio_codec(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        self.root()?;
        self.inner
            .store
            .rollback_extension_package(
                ExtensionPackageKind::AudioCodecProvider.as_str(),
                provider_id,
            )
            .map_err(|error| match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::RollbackUnavailable,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            })?;
        self.load_audio_codec_view(provider_id)
    }

    pub fn remove_plugin(&self, plugin_id: &str) -> Result<(), ExtensionPackageManagementError> {
        self.remove_package(ExtensionPackageKind::OrdinaryPlugin, plugin_id)
    }

    pub fn remove_simulator_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ExtensionPackageManagementError> {
        self.remove_package(ExtensionPackageKind::SimulatorProvider, provider_id)
    }

    pub fn remove_audio_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ExtensionPackageManagementError> {
        self.remove_package(ExtensionPackageKind::AudioProvider, provider_id)
    }

    pub fn remove_audio_codec(
        &self,
        provider_id: &str,
    ) -> Result<(), ExtensionPackageManagementError> {
        self.remove_package(ExtensionPackageKind::AudioCodecProvider, provider_id)
    }

    fn remove_package(
        &self,
        kind: ExtensionPackageKind,
        extension_id: &str,
    ) -> Result<(), ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        let root = self.root()?;
        let state = self
            .inner
            .store
            .load_extension_package_state_record(kind.as_str(), extension_id)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        if state.extension_id != extension_id || !valid_reverse_domain_id(extension_id) {
            return Err(ExtensionPackageManagementError::StorageUnavailable);
        }
        let source = managed_kind_root(root, kind)?.join(extension_id);
        let tombstone_root =
            verified_descendant_directory(root, &root.join(REMOVAL_DIRECTORY).join(kind.as_str()))?;
        let tombstone =
            tombstone_root.join(format!("{}-{}", extension_id, Uuid::new_v4().simple()));
        fs::rename(&source, &tombstone)
            .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
        if self
            .inner
            .store
            .delete_extension_package_records(kind.as_str(), extension_id)
            .is_err()
        {
            let _ = fs::rename(&tombstone, &source);
            return Err(ExtensionPackageManagementError::StorageUnavailable);
        }
        fs::remove_dir_all(tombstone).map_err(|_| ExtensionPackageManagementError::FileOperation)
    }

    fn install_validated_plugin_package(
        &self,
        package: ValidatedPluginPackage,
        source: &str,
        activate: bool,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        let root = self.root()?;
        let package_kind = package.package_manifest.kind.as_str();
        let package_root = managed_kind_root(root, package.package_manifest.kind)?;
        let staging_root = verified_descendant_directory(root, &root.join(STAGING_DIRECTORY))?;
        let plugin_id = package.package_manifest.id.clone();
        let version = package.package_manifest.version.clone();
        let activate = activate
            || (source == EXTENSION_PACKAGE_SOURCE_FIRST_PARTY
                && self.first_party_seed_should_activate(package_kind, &plugin_id, &version)?);
        if let Some(existing) = self
            .inner
            .store
            .load_extension_package_version_record(package_kind, &plugin_id, &version)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            && existing.archive_sha256 != package.archive_sha256
        {
            return Err(ExtensionPackageManagementError::VersionConflict);
        }

        let staging = staging_root.join(Uuid::new_v4().simple().to_string());
        package.extract_to(&staging)?;
        let destination = package_root
            .join(&plugin_id)
            .join(&version)
            .join(&package.archive_sha256);
        let created_destination = if destination.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            false
        } else {
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or(ExtensionPackageManagementError::FileOperation)?,
            )
            .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            fs::rename(&staging, &destination)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            true
        };
        let record = NewExtensionPackageVersionRecord {
            package_kind: package_kind.to_owned(),
            extension_id: plugin_id.clone(),
            version,
            archive_sha256: package.archive_sha256.clone(),
            package_schema_version: package.package_manifest.schema_version,
            source: source.to_owned(),
            package_manifest_json: serde_json::to_string(&package.package_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
            extension_manifest_json: serde_json::to_string(&package.plugin_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
        };
        let saved = if activate {
            self.inner
                .store
                .activate_extension_package_version_record(&record)
        } else {
            self.inner
                .store
                .seed_extension_package_version_record(&record)
        };
        if let Err(error) = saved {
            if created_destination {
                let _ = fs::remove_dir_all(&destination);
            }
            return Err(match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::VersionConflict,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            });
        }
        self.load_plugin_view(&plugin_id)
    }

    fn install_validated_simulator_provider_package(
        &self,
        package: ValidatedSimulatorProviderPackage,
        source: &str,
        activate: bool,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        let root = self.root()?;
        let package_kind = package.package_manifest.kind.as_str();
        let package_root = managed_kind_root(root, package.package_manifest.kind)?;
        let staging_root = verified_descendant_directory(root, &root.join(STAGING_DIRECTORY))?;
        let provider_id = package.package_manifest.id.clone();
        let version = package.package_manifest.version.clone();
        let activate = activate
            || (source == EXTENSION_PACKAGE_SOURCE_FIRST_PARTY
                && self.first_party_seed_should_activate(package_kind, &provider_id, &version)?);
        if let Some(existing) = self
            .inner
            .store
            .load_extension_package_version_record(package_kind, &provider_id, &version)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            && existing.archive_sha256 != package.archive_sha256
        {
            return Err(ExtensionPackageManagementError::VersionConflict);
        }

        let staging = staging_root.join(Uuid::new_v4().simple().to_string());
        package.extract_to(&staging)?;
        let destination = package_root
            .join(&provider_id)
            .join(&version)
            .join(&package.archive_sha256);
        let created_destination = if destination.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            false
        } else {
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or(ExtensionPackageManagementError::FileOperation)?,
            )
            .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            fs::rename(&staging, &destination)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            true
        };
        let record = NewExtensionPackageVersionRecord {
            package_kind: package_kind.to_owned(),
            extension_id: provider_id.clone(),
            version,
            archive_sha256: package.archive_sha256.clone(),
            package_schema_version: package.package_manifest.schema_version,
            source: source.to_owned(),
            package_manifest_json: serde_json::to_string(&package.package_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
            extension_manifest_json: serde_json::to_string(&package.provider_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
        };
        let saved = if activate {
            self.inner
                .store
                .activate_extension_package_version_record(&record)
        } else {
            self.inner
                .store
                .seed_extension_package_version_record(&record)
        };
        if let Err(error) = saved {
            if created_destination {
                let _ = fs::remove_dir_all(&destination);
            }
            return Err(match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::VersionConflict,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            });
        }
        self.load_simulator_provider_view(&provider_id)
    }

    fn install_validated_audio_provider_package(
        &self,
        package: ValidatedAudioProviderPackage,
        source: &str,
        activate: bool,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        let root = self.root()?;
        let package_kind = package.package_manifest.kind.as_str();
        let package_root = managed_kind_root(root, package.package_manifest.kind)?;
        let staging_root = verified_descendant_directory(root, &root.join(STAGING_DIRECTORY))?;
        let provider_id = package.package_manifest.id.clone();
        let version = package.package_manifest.version.clone();
        let activate = activate
            || (source == EXTENSION_PACKAGE_SOURCE_FIRST_PARTY
                && self.first_party_seed_should_activate(package_kind, &provider_id, &version)?);
        if let Some(existing) = self
            .inner
            .store
            .load_extension_package_version_record(package_kind, &provider_id, &version)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            && existing.archive_sha256 != package.archive_sha256
        {
            return Err(ExtensionPackageManagementError::VersionConflict);
        }

        let staging = staging_root.join(Uuid::new_v4().simple().to_string());
        package.extract_to(&staging)?;
        let destination = package_root
            .join(&provider_id)
            .join(&version)
            .join(&package.archive_sha256);
        let created_destination = if destination.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            false
        } else {
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or(ExtensionPackageManagementError::FileOperation)?,
            )
            .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            fs::rename(&staging, &destination)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            true
        };
        let record = NewExtensionPackageVersionRecord {
            package_kind: package_kind.to_owned(),
            extension_id: provider_id.clone(),
            version,
            archive_sha256: package.archive_sha256.clone(),
            package_schema_version: package.package_manifest.schema_version,
            source: source.to_owned(),
            package_manifest_json: serde_json::to_string(&package.package_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
            extension_manifest_json: serde_json::to_string(&package.provider_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
        };
        let saved = if activate {
            self.inner
                .store
                .activate_extension_package_version_record(&record)
        } else {
            self.inner
                .store
                .seed_extension_package_version_record(&record)
        };
        if let Err(error) = saved {
            if created_destination {
                let _ = fs::remove_dir_all(&destination);
            }
            return Err(match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::VersionConflict,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            });
        }
        self.load_audio_provider_view(&provider_id)
    }

    fn install_validated_audio_codec_package(
        &self,
        package: ValidatedAudioCodecPackage,
        source: &str,
        activate: bool,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let _guard = self
            .inner
            .mutation_lock
            .lock()
            .map_err(|_| ExtensionPackageManagementError::StateUnavailable)?;
        let root = self.root()?;
        let package_kind = package.package_manifest.kind.as_str();
        let package_root = managed_kind_root(root, package.package_manifest.kind)?;
        let staging_root = verified_descendant_directory(root, &root.join(STAGING_DIRECTORY))?;
        let provider_id = package.package_manifest.id.clone();
        let version = package.package_manifest.version.clone();
        let activate = activate
            || (source == EXTENSION_PACKAGE_SOURCE_FIRST_PARTY
                && self.first_party_seed_should_activate(package_kind, &provider_id, &version)?);
        if let Some(existing) = self
            .inner
            .store
            .load_extension_package_version_record(package_kind, &provider_id, &version)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            && existing.archive_sha256 != package.archive_sha256
        {
            return Err(ExtensionPackageManagementError::VersionConflict);
        }

        let staging = staging_root.join(Uuid::new_v4().simple().to_string());
        package.extract_to(&staging)?;
        let destination = package_root
            .join(&provider_id)
            .join(&version)
            .join(&package.archive_sha256);
        let created_destination = if destination.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            false
        } else {
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or(ExtensionPackageManagementError::FileOperation)?,
            )
            .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            fs::rename(&staging, &destination)
                .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
            true
        };
        let record = NewExtensionPackageVersionRecord {
            package_kind: package_kind.to_owned(),
            extension_id: provider_id.clone(),
            version,
            archive_sha256: package.archive_sha256.clone(),
            package_schema_version: package.package_manifest.schema_version,
            source: source.to_owned(),
            package_manifest_json: serde_json::to_string(&package.package_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
            extension_manifest_json: serde_json::to_string(&package.codec_manifest)
                .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?,
        };
        let saved = if activate {
            self.inner
                .store
                .activate_extension_package_version_record(&record)
        } else {
            self.inner
                .store
                .seed_extension_package_version_record(&record)
        };
        if let Err(error) = saved {
            if created_destination {
                let _ = fs::remove_dir_all(&destination);
            }
            return Err(match error {
                StorageError::InvalidRecord => ExtensionPackageManagementError::VersionConflict,
                _ => ExtensionPackageManagementError::StorageUnavailable,
            });
        }
        self.load_audio_codec_view(&provider_id)
    }

    fn first_party_seed_should_activate(
        &self,
        package_kind: &str,
        plugin_id: &str,
        version: &str,
    ) -> Result<bool, ExtensionPackageManagementError> {
        let Some(state) = self
            .inner
            .store
            .load_extension_package_state_record(package_kind, plugin_id)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
        else {
            return Ok(false);
        };
        let active = self
            .inner
            .store
            .load_extension_package_version_record(package_kind, plugin_id, &state.active_version)
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        Ok(active.source == EXTENSION_PACKAGE_SOURCE_FIRST_PARTY
            && semantic_version_is_newer(version, &state.active_version))
    }

    fn load_plugin_view(
        &self,
        plugin_id: &str,
    ) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
        let root = self.root()?;
        let state = self
            .inner
            .store
            .load_extension_package_state_record(
                ExtensionPackageKind::OrdinaryPlugin.as_str(),
                plugin_id,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        managed_plugin_view(&self.inner.store, root, state)
    }

    fn load_simulator_provider_view(
        &self,
        provider_id: &str,
    ) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
        let root = self.root()?;
        let state = self
            .inner
            .store
            .load_extension_package_state_record(
                ExtensionPackageKind::SimulatorProvider.as_str(),
                provider_id,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        managed_simulator_provider_view(&self.inner.store, root, state)
    }

    fn load_audio_provider_view(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
        let root = self.root()?;
        let state = self
            .inner
            .store
            .load_extension_package_state_record(
                ExtensionPackageKind::AudioProvider.as_str(),
                provider_id,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        managed_audio_provider_view(&self.inner.store, root, state)
    }

    fn load_audio_codec_view(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
        let root = self.root()?;
        let state = self
            .inner
            .store
            .load_extension_package_state_record(
                ExtensionPackageKind::AudioCodecProvider.as_str(),
                provider_id,
            )
            .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?
            .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
        managed_audio_codec_view(&self.inner.store, root, state)
    }

    fn root(&self) -> Result<&Path, ExtensionPackageManagementError> {
        let root = self
            .inner
            .root
            .as_deref()
            .ok_or(ExtensionPackageManagementError::RootUnavailable)?;
        let metadata = fs::symlink_metadata(root)
            .map_err(|_| ExtensionPackageManagementError::RootUnavailable)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ExtensionPackageManagementError::RootUnavailable);
        }
        let canonical = root
            .canonicalize()
            .map_err(|_| ExtensionPackageManagementError::RootUnavailable)?;
        if canonical != root {
            return Err(ExtensionPackageManagementError::RootUnavailable);
        }
        Ok(root)
    }
}

pub(crate) struct ValidatedPluginPackage {
    bytes: Vec<u8>,
    pub package_manifest: ExtensionPackageManifest,
    pub plugin_manifest: PluginManifest,
    pub archive_sha256: String,
}

impl ValidatedPluginPackage {
    pub fn inspection(&self) -> PluginPackageInspection {
        PluginPackageInspection {
            package_schema_version: self.package_manifest.schema_version,
            package_kind: self.package_manifest.kind,
            id: self.package_manifest.id.clone(),
            name: self.plugin_manifest.name.clone(),
            version: self.package_manifest.version.clone(),
            author: self.plugin_manifest.author.clone(),
            runtime: self.plugin_manifest.runtime,
            archive_sha256: self.archive_sha256.clone(),
            archive_size: self.bytes.len() as u64,
            expanded_size: self.package_manifest.expanded_size(),
            file_count: self.package_manifest.files.len(),
            publisher_verified: false,
        }
    }

    pub fn extract_to(&self, destination: &Path) -> Result<(), ExtensionPackageError> {
        extract_validated_package(&self.bytes, &self.package_manifest, destination)
    }
}

pub(crate) struct ValidatedSimulatorProviderPackage {
    bytes: Vec<u8>,
    pub package_manifest: ExtensionPackageManifest,
    pub provider_manifest: ProviderManifest,
    pub archive_sha256: String,
}

impl ValidatedSimulatorProviderPackage {
    pub fn inspection(&self) -> SimulatorProviderPackageInspection {
        SimulatorProviderPackageInspection {
            package_schema_version: self.package_manifest.schema_version,
            package_kind: self.package_manifest.kind,
            id: self.package_manifest.id.clone(),
            name: self.provider_manifest.name.clone(),
            version: self.package_manifest.version.clone(),
            author: self.provider_manifest.author.clone(),
            bridge_protocol_version: self.provider_manifest.bridge_protocol_version,
            platforms: self.provider_manifest.platforms.clone(),
            simulators: self.provider_manifest.simulators.clone(),
            capabilities: self.provider_manifest.capabilities.clone(),
            archive_sha256: self.archive_sha256.clone(),
            archive_size: self.bytes.len() as u64,
            expanded_size: self.package_manifest.expanded_size(),
            file_count: self.package_manifest.files.len(),
            publisher_verified: false,
        }
    }

    pub fn extract_to(&self, destination: &Path) -> Result<(), ExtensionPackageError> {
        extract_validated_package(&self.bytes, &self.package_manifest, destination)?;
        set_provider_entry_point_executable(destination, &self.provider_manifest.entry_point)
    }
}

pub(crate) struct ValidatedAudioProviderPackage {
    bytes: Vec<u8>,
    pub package_manifest: ExtensionPackageManifest,
    pub provider_manifest: AudioProviderManifest,
    pub archive_sha256: String,
}

impl ValidatedAudioProviderPackage {
    pub fn inspection(&self) -> AudioProviderPackageInspection {
        AudioProviderPackageInspection {
            package_schema_version: self.package_manifest.schema_version,
            package_kind: self.package_manifest.kind,
            id: self.package_manifest.id.clone(),
            name: self.provider_manifest.name.clone(),
            version: self.package_manifest.version.clone(),
            author: self.provider_manifest.author.clone(),
            audio_protocol_version: self.provider_manifest.audio_protocol_version,
            platforms: self.provider_manifest.platforms.clone(),
            capabilities: self.provider_manifest.capabilities.clone(),
            archive_sha256: self.archive_sha256.clone(),
            archive_size: self.bytes.len() as u64,
            expanded_size: self.package_manifest.expanded_size(),
            file_count: self.package_manifest.files.len(),
            publisher_verified: false,
        }
    }

    pub fn extract_to(&self, destination: &Path) -> Result<(), ExtensionPackageError> {
        extract_validated_package(&self.bytes, &self.package_manifest, destination)?;
        if self
            .provider_manifest
            .platforms
            .contains(&current_audio_provider_platform())
        {
            let entry_point = provider_executable_in(destination, &self.provider_manifest);
            set_provider_entry_point_executable_path(&entry_point)
        } else {
            Ok(())
        }
    }
}

pub(crate) struct ValidatedAudioCodecPackage {
    bytes: Vec<u8>,
    pub package_manifest: ExtensionPackageManifest,
    pub codec_manifest: AudioCodecManifest,
    pub archive_sha256: String,
}

impl ValidatedAudioCodecPackage {
    pub fn inspection(&self) -> AudioCodecPackageInspection {
        AudioCodecPackageInspection {
            package_schema_version: self.package_manifest.schema_version,
            package_kind: self.package_manifest.kind,
            id: self.package_manifest.id.clone(),
            name: self.codec_manifest.name.clone(),
            version: self.package_manifest.version.clone(),
            author: self.codec_manifest.author.clone(),
            codec_protocol_version: self.codec_manifest.codec_protocol_version,
            platforms: self.codec_manifest.platforms.clone(),
            capabilities: self.codec_manifest.capabilities.clone(),
            profiles: self.codec_manifest.profiles.clone(),
            archive_sha256: self.archive_sha256.clone(),
            archive_size: self.bytes.len() as u64,
            expanded_size: self.package_manifest.expanded_size(),
            file_count: self.package_manifest.files.len(),
            publisher_verified: false,
        }
    }

    pub fn extract_to(&self, destination: &Path) -> Result<(), ExtensionPackageError> {
        extract_validated_package(&self.bytes, &self.package_manifest, destination)?;
        if self
            .codec_manifest
            .platforms
            .contains(&current_codec_platform())
        {
            let entry_point = codec_executable_in(destination, &self.codec_manifest);
            set_provider_entry_point_executable_path(&entry_point)
        } else {
            Ok(())
        }
    }
}

pub fn inspect_plugin_package(
    path: &Path,
) -> Result<PluginPackageInspection, ExtensionPackageError> {
    read_plugin_package(path).map(|package| package.inspection())
}

pub fn inspect_simulator_provider_package(
    path: &Path,
) -> Result<SimulatorProviderPackageInspection, ExtensionPackageError> {
    read_simulator_provider_package(path).map(|package| package.inspection())
}

pub fn inspect_audio_provider_package(
    path: &Path,
) -> Result<AudioProviderPackageInspection, ExtensionPackageError> {
    read_audio_provider_package(path).map(|package| package.inspection())
}

pub fn inspect_audio_codec_package(
    path: &Path,
) -> Result<AudioCodecPackageInspection, ExtensionPackageError> {
    read_audio_codec_package(path).map(|package| package.inspection())
}

pub(crate) fn read_plugin_package(
    path: &Path,
) -> Result<ValidatedPluginPackage, ExtensionPackageError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Unavailable);
    }
    if metadata.len() > MAX_PACKAGE_ARCHIVE_BYTES {
        return Err(ExtensionPackageError::ArchiveTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    let archive_sha256 = hex_sha256(&bytes);
    let (package_manifest, manifest_json) =
        validate_package_bytes(&bytes, ExtensionPackageKind::OrdinaryPlugin)?;
    let plugin_manifest: PluginManifest = serde_json::from_slice(&manifest_json)
        .map_err(|_| ExtensionPackageError::InvalidPluginManifest)?;
    plugin_manifest
        .validate()
        .map_err(|_| ExtensionPackageError::InvalidPluginManifest)?;
    if plugin_manifest.id != package_manifest.id
        || plugin_manifest.version != package_manifest.version
        || !package_manifest
            .files
            .iter()
            .any(|file| file.path == plugin_manifest.entry_point)
    {
        return Err(ExtensionPackageError::InvalidPluginManifest);
    }
    Ok(ValidatedPluginPackage {
        bytes,
        package_manifest,
        plugin_manifest,
        archive_sha256,
    })
}

pub(crate) fn read_simulator_provider_package(
    path: &Path,
) -> Result<ValidatedSimulatorProviderPackage, ExtensionPackageError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Unavailable);
    }
    if metadata.len() > MAX_PACKAGE_ARCHIVE_BYTES {
        return Err(ExtensionPackageError::ArchiveTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    let archive_sha256 = hex_sha256(&bytes);
    let (package_manifest, manifest_json) =
        validate_package_bytes(&bytes, ExtensionPackageKind::SimulatorProvider)?;
    let provider_manifest: ProviderManifest = serde_json::from_slice(&manifest_json)
        .map_err(|_| ExtensionPackageError::InvalidSimulatorProviderManifest)?;
    provider_manifest
        .validate()
        .map_err(|_| ExtensionPackageError::InvalidSimulatorProviderManifest)?;
    if provider_manifest.id != package_manifest.id
        || provider_manifest.version != package_manifest.version
        || provider_manifest.entry_point == package_manifest.manifest_path
        || package_manifest.files.len() < 2
        || !package_manifest
            .files
            .iter()
            .any(|file| file.path == provider_manifest.entry_point)
    {
        return Err(ExtensionPackageError::InvalidSimulatorProviderManifest);
    }
    Ok(ValidatedSimulatorProviderPackage {
        bytes,
        package_manifest,
        provider_manifest,
        archive_sha256,
    })
}

pub(crate) fn read_audio_provider_package(
    path: &Path,
) -> Result<ValidatedAudioProviderPackage, ExtensionPackageError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Unavailable);
    }
    if metadata.len() > MAX_PACKAGE_ARCHIVE_BYTES {
        return Err(ExtensionPackageError::ArchiveTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    let archive_sha256 = hex_sha256(&bytes);
    let (package_manifest, manifest_json) =
        validate_package_bytes(&bytes, ExtensionPackageKind::AudioProvider)?;
    let provider_manifest: AudioProviderManifest = serde_json::from_slice(&manifest_json)
        .map_err(|_| ExtensionPackageError::InvalidAudioProviderManifest)?;
    provider_manifest
        .validate()
        .map_err(|_| ExtensionPackageError::InvalidAudioProviderManifest)?;
    if provider_manifest.id != package_manifest.id
        || provider_manifest.version != package_manifest.version
        || provider_manifest.entry_point == package_manifest.manifest_path
        || package_manifest.files.len() < 2
        || !audio_provider_entry_points(&provider_manifest)
            .iter()
            .all(|entry_point| {
                package_manifest
                    .files
                    .iter()
                    .any(|file| &file.path == entry_point)
            })
    {
        return Err(ExtensionPackageError::InvalidAudioProviderManifest);
    }
    Ok(ValidatedAudioProviderPackage {
        bytes,
        package_manifest,
        provider_manifest,
        archive_sha256,
    })
}

pub(crate) fn read_audio_codec_package(
    path: &Path,
) -> Result<ValidatedAudioCodecPackage, ExtensionPackageError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Unavailable);
    }
    if metadata.len() > MAX_PACKAGE_ARCHIVE_BYTES {
        return Err(ExtensionPackageError::ArchiveTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| ExtensionPackageError::Unavailable)?;
    let archive_sha256 = hex_sha256(&bytes);
    let (package_manifest, manifest_json) =
        validate_package_bytes(&bytes, ExtensionPackageKind::AudioCodecProvider)?;
    let codec_manifest: AudioCodecManifest = serde_json::from_slice(&manifest_json)
        .map_err(|_| ExtensionPackageError::InvalidAudioCodecManifest)?;
    codec_manifest
        .validate()
        .map_err(|_| ExtensionPackageError::InvalidAudioCodecManifest)?;
    if codec_manifest.id != package_manifest.id
        || codec_manifest.version != package_manifest.version
        || codec_manifest.entry_point == package_manifest.manifest_path
        || package_manifest.files.len() < 2
        || !audio_codec_entry_points(&codec_manifest)
            .iter()
            .all(|entry_point| {
                package_manifest
                    .files
                    .iter()
                    .any(|file| &file.path == entry_point)
            })
    {
        return Err(ExtensionPackageError::InvalidAudioCodecManifest);
    }
    Ok(ValidatedAudioCodecPackage {
        bytes,
        package_manifest,
        codec_manifest,
        archive_sha256,
    })
}

fn set_provider_entry_point_executable(
    destination: &Path,
    entry_point: &str,
) -> Result<(), ExtensionPackageError> {
    set_provider_entry_point_executable_path(&destination.join(entry_point))
}

fn set_provider_entry_point_executable_path(
    entry_point: &Path,
) -> Result<(), ExtensionPackageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(entry_point)
            .map_err(|_| ExtensionPackageError::ExtractionFailed)?
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(entry_point, permissions)
            .map_err(|_| ExtensionPackageError::ExtractionFailed)?;
    }
    #[cfg(not(unix))]
    let _ = entry_point;
    Ok(())
}

fn audio_provider_entry_points(manifest: &AudioProviderManifest) -> Vec<String> {
    let mut entry_points = BTreeSet::new();
    for platform in &manifest.platforms {
        let mut entry_point = manifest.entry_point.clone();
        if *platform == AudioProviderPlatform::WindowsX86_64 {
            entry_point.push_str(".exe");
        }
        entry_points.insert(entry_point);
    }
    entry_points.into_iter().collect()
}

fn audio_codec_entry_points(manifest: &AudioCodecManifest) -> Vec<String> {
    let mut entry_points = BTreeSet::new();
    for platform in &manifest.platforms {
        let mut entry_point = manifest.entry_point.clone();
        if *platform == AudioCodecPlatform::WindowsX86_64 {
            entry_point.push_str(".exe");
        }
        entry_points.insert(entry_point);
    }
    entry_points.into_iter().collect()
}

fn extract_validated_package(
    bytes: &[u8],
    package_manifest: &ExtensionPackageManifest,
    destination: &Path,
) -> Result<(), ExtensionPackageError> {
    fs::create_dir_all(destination).map_err(|_| ExtensionPackageError::ExtractionFailed)?;
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| ExtensionPackageError::InvalidArchive)?;
    for declared in &package_manifest.files {
        let mut archived = archive
            .by_name(&declared.path)
            .map_err(|_| ExtensionPackageError::PayloadMismatch)?;
        validate_archive_entry(&archived, declared.size)?;
        let path = destination.join(Path::new(&declared.path));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| ExtensionPackageError::ExtractionFailed)?;
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|_| ExtensionPackageError::ExtractionFailed)?;
        let copied = std::io::copy(&mut archived, &mut output)
            .map_err(|_| ExtensionPackageError::ExtractionFailed)?;
        output
            .flush()
            .map_err(|_| ExtensionPackageError::ExtractionFailed)?;
        if copied != declared.size {
            return Err(ExtensionPackageError::PayloadMismatch);
        }
    }
    Ok(())
}

fn prepare_package_root(root: PathBuf) -> Result<PathBuf, std::io::Error> {
    ensure_directory(&root)?;
    ensure_directory(&root.join(STAGING_DIRECTORY))?;
    ensure_directory(&root.join(REMOVAL_DIRECTORY))?;
    for kind in [
        ExtensionPackageKind::OrdinaryPlugin,
        ExtensionPackageKind::SimulatorProvider,
        ExtensionPackageKind::AudioProvider,
        ExtensionPackageKind::AudioCodecProvider,
    ] {
        ensure_directory(&root.join(kind.as_str()))?;
        ensure_directory(&root.join(REMOVAL_DIRECTORY).join(kind.as_str()))?;
    }
    let root = root.canonicalize()?;
    let staging = root.join(STAGING_DIRECTORY);
    for entry in fs::read_dir(&staging)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() && !file_type.is_symlink() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(root)
}

fn ensure_directory(path: &Path) -> Result<(), std::io::Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "unsafe package directory"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

fn recover_pending_removals(root: &Path, store: &Store) -> Result<(), std::io::Error> {
    for kind in [
        ExtensionPackageKind::OrdinaryPlugin,
        ExtensionPackageKind::SimulatorProvider,
        ExtensionPackageKind::AudioProvider,
        ExtensionPackageKind::AudioCodecProvider,
    ] {
        recover_pending_removals_for_kind(root, store, kind)?;
    }
    Ok(())
}

fn recover_pending_removals_for_kind(
    root: &Path,
    store: &Store,
    kind: ExtensionPackageKind,
) -> Result<(), std::io::Error> {
    let removal_root = root.join(REMOVAL_DIRECTORY).join(kind.as_str());
    for entry in fs::read_dir(removal_root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() || file_type.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid extension removal tombstone",
            ));
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid extension removal tombstone name",
            )
        })?;
        let split = name.len().checked_sub(33).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid extension removal tombstone name",
            )
        })?;
        if name.as_bytes().get(split) != Some(&b'-') || Uuid::parse_str(&name[split + 1..]).is_err()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid extension removal tombstone name",
            ));
        }
        let extension_id = &name[..split];
        if !valid_reverse_domain_id(extension_id) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid extension removal tombstone identity",
            ));
        }
        let installed = store
            .load_extension_package_state_record(kind.as_str(), extension_id)
            .map_err(|_| std::io::Error::other("extension storage unavailable"))?
            .is_some();
        let destination = root.join(kind.as_str()).join(extension_id);
        if installed && !destination.exists() {
            fs::rename(entry.path(), destination)?;
        } else {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn managed_payload_root(
    root: &Path,
    kind: ExtensionPackageKind,
    extension_id: &str,
    version: &str,
    archive_sha256: &str,
) -> Result<PathBuf, ExtensionPackageManagementError> {
    if !valid_reverse_domain_id(extension_id)
        || !valid_semantic_version(version)
        || !valid_sha256(archive_sha256)
    {
        return Err(ExtensionPackageManagementError::StorageUnavailable);
    }
    let managed_root = managed_kind_root(root, kind)?;
    let payload = managed_root
        .join(extension_id)
        .join(version)
        .join(archive_sha256)
        .canonicalize()
        .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
    if !payload.starts_with(&managed_root) || !payload.is_dir() {
        return Err(ExtensionPackageManagementError::FileOperation);
    }
    Ok(payload)
}

fn managed_kind_root(
    root: &Path,
    kind: ExtensionPackageKind,
) -> Result<PathBuf, ExtensionPackageManagementError> {
    verified_descendant_directory(root, &root.join(kind.as_str()))
}

fn verified_descendant_directory(
    root: &Path,
    path: &Path,
) -> Result<PathBuf, ExtensionPackageManagementError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ExtensionPackageManagementError::FileOperation)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ExtensionPackageManagementError::FileOperation);
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| ExtensionPackageManagementError::FileOperation)?;
    if canonical == root || !canonical.starts_with(root) {
        return Err(ExtensionPackageManagementError::FileOperation);
    }
    Ok(canonical)
}

fn managed_plugin_view(
    store: &Store,
    root: &Path,
    state: wyrmgrid_storage::ExtensionPackageStateRecord,
) -> Result<ManagedPluginPackageView, ExtensionPackageManagementError> {
    let versions = store
        .list_extension_package_version_records(&state.package_kind, &state.extension_id)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    let active = versions
        .iter()
        .find(|version| version.version == state.active_version)
        .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
    let manifest: PluginManifest = serde_json::from_str(&active.extension_manifest_json)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    manifest
        .validate()
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    if manifest.id != state.extension_id || manifest.version != state.active_version {
        return Err(ExtensionPackageManagementError::StorageUnavailable);
    }
    managed_payload_root(
        root,
        ExtensionPackageKind::OrdinaryPlugin,
        &state.extension_id,
        &state.active_version,
        &active.archive_sha256,
    )?;
    Ok(ManagedPluginPackageView {
        id: state.extension_id,
        name: manifest.name,
        author: manifest.author,
        active_version: state.active_version,
        rollback_version: state.rollback_version,
        enabled: state.enabled,
        installed_versions: versions
            .iter()
            .map(|version| version.version.clone())
            .collect(),
        active_archive_sha256: active.archive_sha256.clone(),
        source: active.source.clone(),
        publisher_verified: false,
    })
}

fn managed_simulator_provider_view(
    store: &Store,
    root: &Path,
    state: wyrmgrid_storage::ExtensionPackageStateRecord,
) -> Result<ManagedSimulatorProviderPackageView, ExtensionPackageManagementError> {
    let versions = store
        .list_extension_package_version_records(&state.package_kind, &state.extension_id)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    let active = versions
        .iter()
        .find(|version| version.version == state.active_version)
        .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
    let manifest: ProviderManifest = serde_json::from_str(&active.extension_manifest_json)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    manifest
        .validate()
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    if manifest.id != state.extension_id || manifest.version != state.active_version {
        return Err(ExtensionPackageManagementError::StorageUnavailable);
    }
    managed_payload_root(
        root,
        ExtensionPackageKind::SimulatorProvider,
        &state.extension_id,
        &state.active_version,
        &active.archive_sha256,
    )?;
    Ok(ManagedSimulatorProviderPackageView {
        id: state.extension_id,
        name: manifest.name,
        author: manifest.author,
        active_version: state.active_version,
        rollback_version: state.rollback_version,
        enabled: state.enabled,
        installed_versions: versions
            .iter()
            .map(|version| version.version.clone())
            .collect(),
        active_archive_sha256: active.archive_sha256.clone(),
        source: active.source.clone(),
        publisher_verified: false,
        bridge_protocol_version: manifest.bridge_protocol_version,
        platforms: manifest.platforms,
        simulators: manifest.simulators,
        capabilities: manifest.capabilities,
    })
}

fn managed_audio_provider_view(
    store: &Store,
    root: &Path,
    state: wyrmgrid_storage::ExtensionPackageStateRecord,
) -> Result<ManagedAudioProviderPackageView, ExtensionPackageManagementError> {
    let versions = store
        .list_extension_package_version_records(&state.package_kind, &state.extension_id)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    let active = versions
        .iter()
        .find(|version| version.version == state.active_version)
        .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
    let manifest: AudioProviderManifest = serde_json::from_str(&active.extension_manifest_json)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    manifest
        .validate()
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    if manifest.id != state.extension_id || manifest.version != state.active_version {
        return Err(ExtensionPackageManagementError::StorageUnavailable);
    }
    managed_payload_root(
        root,
        ExtensionPackageKind::AudioProvider,
        &state.extension_id,
        &state.active_version,
        &active.archive_sha256,
    )?;
    Ok(ManagedAudioProviderPackageView {
        id: state.extension_id,
        name: manifest.name,
        author: manifest.author,
        active_version: state.active_version,
        rollback_version: state.rollback_version,
        enabled: state.enabled,
        installed_versions: versions
            .iter()
            .map(|version| version.version.clone())
            .collect(),
        active_archive_sha256: active.archive_sha256.clone(),
        source: active.source.clone(),
        publisher_verified: false,
        audio_protocol_version: manifest.audio_protocol_version,
        platforms: manifest.platforms,
        capabilities: manifest.capabilities,
    })
}

fn managed_audio_codec_view(
    store: &Store,
    root: &Path,
    state: wyrmgrid_storage::ExtensionPackageStateRecord,
) -> Result<ManagedAudioCodecPackageView, ExtensionPackageManagementError> {
    let versions = store
        .list_extension_package_version_records(&state.package_kind, &state.extension_id)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    let active = versions
        .iter()
        .find(|version| version.version == state.active_version)
        .ok_or(ExtensionPackageManagementError::StorageUnavailable)?;
    let manifest: AudioCodecManifest = serde_json::from_str(&active.extension_manifest_json)
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    manifest
        .validate()
        .map_err(|_| ExtensionPackageManagementError::StorageUnavailable)?;
    if manifest.id != state.extension_id || manifest.version != state.active_version {
        return Err(ExtensionPackageManagementError::StorageUnavailable);
    }
    managed_payload_root(
        root,
        ExtensionPackageKind::AudioCodecProvider,
        &state.extension_id,
        &state.active_version,
        &active.archive_sha256,
    )?;
    Ok(ManagedAudioCodecPackageView {
        id: state.extension_id,
        name: manifest.name,
        author: manifest.author,
        active_version: state.active_version,
        rollback_version: state.rollback_version,
        enabled: state.enabled,
        installed_versions: versions
            .iter()
            .map(|version| version.version.clone())
            .collect(),
        active_archive_sha256: active.archive_sha256.clone(),
        source: active.source.clone(),
        publisher_verified: false,
        codec_protocol_version: manifest.codec_protocol_version,
        platforms: manifest.platforms,
        capabilities: manifest.capabilities,
        profiles: manifest.profiles,
    })
}

fn validate_package_bytes(
    bytes: &[u8],
    expected_kind: ExtensionPackageKind,
) -> Result<(ExtensionPackageManifest, Vec<u8>), ExtensionPackageError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| ExtensionPackageError::InvalidArchive)?;
    if archive.len() < 2 || archive.len() > MAX_PACKAGE_FILES + 1 {
        return Err(ExtensionPackageError::InvalidArchive);
    }

    let mut archive_paths = BTreeSet::new();
    let mut case_folded_paths = BTreeSet::new();
    for index in 0..archive.len() {
        let archived = archive
            .by_index(index)
            .map_err(|_| ExtensionPackageError::InvalidArchive)?;
        let name = archived.name();
        if !valid_package_path(name)
            || !archive_paths.insert(name.to_owned())
            || !case_folded_paths.insert(name.to_ascii_lowercase())
        {
            return Err(ExtensionPackageError::UnsafeArchiveEntry);
        }
        validate_archive_entry(
            &archived,
            if name == EXTENSION_PACKAGE_MANIFEST_NAME {
                archived.size()
            } else {
                archived.size().min(MAX_PACKAGE_FILE_BYTES)
            },
        )?;
    }

    let manifest_json = {
        let mut archived = archive
            .by_name(EXTENSION_PACKAGE_MANIFEST_NAME)
            .map_err(|_| ExtensionPackageError::InvalidArchive)?;
        if archived.size() == 0 || archived.size() > MAX_PACKAGE_MANIFEST_BYTES {
            return Err(ExtensionPackageError::InvalidArchive);
        }
        let mut bytes = Vec::with_capacity(archived.size() as usize);
        archived
            .read_to_end(&mut bytes)
            .map_err(|_| ExtensionPackageError::InvalidArchive)?;
        String::from_utf8(bytes).map_err(|_| ExtensionPackageError::InvalidArchive)?
    };
    let package_manifest: ExtensionPackageManifest =
        serde_json::from_str(&manifest_json).map_err(|_| ExtensionPackageError::InvalidArchive)?;
    package_manifest.validate()?;
    if package_manifest.kind != expected_kind {
        return Err(ExtensionPackageError::UnsupportedPackageKind);
    }

    let declared = package_manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let expected_paths = declared
        .keys()
        .map(|path| (*path).to_owned())
        .chain(std::iter::once(EXTENSION_PACKAGE_MANIFEST_NAME.to_owned()))
        .collect::<BTreeSet<_>>();
    if archive_paths != expected_paths {
        return Err(ExtensionPackageError::InvalidInventory);
    }

    let mut extension_manifest_json = None;
    for (path, declared) in declared {
        let mut archived = archive
            .by_name(path)
            .map_err(|_| ExtensionPackageError::PayloadMismatch)?;
        validate_archive_entry(&archived, declared.size)?;
        if archived.size() != declared.size {
            return Err(ExtensionPackageError::PayloadMismatch);
        }
        let mut hasher = Sha256::new();
        let mut contents = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        let mut read_bytes = 0_u64;
        loop {
            let count = archived
                .read(&mut buffer)
                .map_err(|_| ExtensionPackageError::PayloadMismatch)?;
            if count == 0 {
                break;
            }
            read_bytes = read_bytes
                .checked_add(count as u64)
                .ok_or(ExtensionPackageError::PayloadMismatch)?;
            if read_bytes > declared.size {
                return Err(ExtensionPackageError::PayloadMismatch);
            }
            hasher.update(&buffer[..count]);
            if path == package_manifest.manifest_path {
                contents.extend_from_slice(&buffer[..count]);
            }
        }
        if read_bytes != declared.size || hex_digest(&hasher.finalize()) != declared.sha256 {
            return Err(ExtensionPackageError::PayloadMismatch);
        }
        if path == package_manifest.manifest_path {
            extension_manifest_json = Some(contents);
        }
    }
    Ok((
        package_manifest,
        extension_manifest_json.ok_or(ExtensionPackageError::InvalidInventory)?,
    ))
}

fn validate_archive_entry<R: Read>(
    archived: &zip::read::ZipFile<'_, R>,
    maximum_size: u64,
) -> Result<(), ExtensionPackageError> {
    if archived.encrypted()
        || !archived.is_file()
        || archived.size() == 0
        || archived.size() > maximum_size
        || !matches!(
            archived.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        )
        || archived.enclosed_name().is_none()
    {
        return Err(ExtensionPackageError::UnsafeArchiveEntry);
    }
    Ok(())
}

fn valid_package_path(path: &str) -> bool {
    if path.is_empty()
        || path.len() > MAX_PACKAGE_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains("//")
        || path.contains('\\')
        || !path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return false;
    }
    let parsed = Path::new(path);
    let components = parsed.components().collect::<Vec<_>>();
    if components.is_empty() || components.len() > MAX_PACKAGE_PATH_DEPTH {
        return false;
    }
    components.into_iter().all(|component| match component {
        Component::Normal(value) => value.to_str().is_some_and(|value| {
            !value.is_empty()
                && value != "."
                && value != ".."
                && value.len() <= MAX_PACKAGE_COMPONENT_BYTES
                && !reserved_windows_component(value)
        }),
        _ => false,
    })
}

fn reserved_windows_component(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                suffix.len() == 1
                    && suffix
                        .as_bytes()
                        .first()
                        .is_some_and(|digit| matches!(digit, b'1'..=b'9'))
            })
}

fn valid_reverse_domain_id(id: &str) -> bool {
    let segments = id.split('.').collect::<Vec<_>>();
    segments.len() >= 3
        && id.len() <= 255
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= 63
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                && segment
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn valid_semantic_version(version: &str) -> bool {
    let components = version.split('.').collect::<Vec<_>>();
    components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && (component == &"0" || !component.starts_with('0'))
                && component.parse::<u64>().is_ok()
        })
}

fn semantic_version_is_newer(candidate: &str, current: &str) -> bool {
    let parse = |version: &str| {
        let mut components = version.split('.');
        Some((
            components.next()?.parse::<u64>().ok()?,
            components.next()?.parse::<u64>().ok()?,
            components.next()?.parse::<u64>().ok()?,
        ))
    };
    matches!((parse(candidate), parse(current)), (Some(candidate), Some(current)) if candidate > current)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex_sha256(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

#[cfg(test)]
#[path = "tests/extension_packages.rs"]
mod tests;

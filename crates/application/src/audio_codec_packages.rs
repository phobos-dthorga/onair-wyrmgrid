use crate::{
    AudioCodecPackageInspection, AudioCodecProvider, AudioCodecRegistration,
    ExtensionPackageManagementError, ExtensionPackageService, ManagedAudioCodecPackageView,
    ProcessAudioCodecProvider, inspect_audio_codec_package,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecPackageError {
    #[error("The selected audio codec package is invalid or unsupported.")]
    InvalidPackage,
    #[error("Local audio codec package storage is unavailable.")]
    PackageStorageUnavailable,
    #[error("That audio codec version already exists with different package contents.")]
    PackageVersionConflict,
    #[error("No previous audio codec version is available for rollback.")]
    RollbackUnavailable,
    #[error("That audio codec is not installed, enabled, or available on this platform.")]
    ProviderUnavailable,
    #[error("Audio codec package state is unavailable.")]
    StateUnavailable,
}

#[derive(Clone)]
pub struct AudioCodecPackageService {
    inner: Arc<AudioCodecPackageServiceInner>,
}

struct AudioCodecPackageServiceInner {
    packages: ExtensionPackageService,
    runtimes: Mutex<BTreeMap<String, CachedAudioCodec>>,
    mutation: Mutex<()>,
}

struct CachedAudioCodec {
    version: String,
    provider: Arc<dyn AudioCodecProvider>,
}

impl AudioCodecPackageService {
    pub fn new(packages: ExtensionPackageService) -> Self {
        Self {
            inner: Arc::new(AudioCodecPackageServiceInner {
                packages,
                runtimes: Mutex::new(BTreeMap::new()),
                mutation: Mutex::new(()),
            }),
        }
    }

    pub fn inspect_package(
        &self,
        path: &Path,
    ) -> Result<AudioCodecPackageInspection, AudioCodecPackageError> {
        inspect_audio_codec_package(path).map_err(|_| AudioCodecPackageError::InvalidPackage)
    }

    pub fn list_packages(
        &self,
    ) -> Result<Vec<ManagedAudioCodecPackageView>, AudioCodecPackageError> {
        self.inner
            .packages
            .list_audio_codec_packages()
            .map_err(map_package_error)
    }

    pub fn install_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        let installed = self
            .inner
            .packages
            .install_audio_codec_package(path)
            .map_err(map_package_error)?;
        self.invalidate(&installed.id)?;
        Ok(installed)
    }

    pub fn seed_first_party_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        let installed = self
            .inner
            .packages
            .seed_first_party_audio_codec_package(path)
            .map_err(map_package_error)?;
        self.invalidate(&installed.id)?;
        Ok(installed)
    }

    pub fn set_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioCodecPackageView, AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        let view = self
            .inner
            .packages
            .set_audio_codec_enabled(provider_id, enabled)
            .map_err(map_package_error)?;
        self.invalidate(provider_id)?;
        Ok(view)
    }

    pub fn rollback(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioCodecPackageView, AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        let view = self
            .inner
            .packages
            .rollback_audio_codec(provider_id)
            .map_err(map_package_error)?;
        self.invalidate(provider_id)?;
        Ok(view)
    }

    pub fn remove(&self, provider_id: &str) -> Result<(), AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        self.inner
            .packages
            .remove_audio_codec(provider_id)
            .map_err(map_package_error)?;
        self.invalidate(provider_id)
    }

    pub(crate) fn providers(
        &self,
    ) -> Result<Vec<Arc<dyn AudioCodecProvider>>, AudioCodecPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        let managed = self
            .inner
            .packages
            .active_managed_audio_codecs()
            .map_err(map_package_error)?;
        let active_ids = managed
            .iter()
            .map(|codec| codec.manifest.id.clone())
            .collect::<BTreeSet<_>>();
        let mut runtimes = self
            .inner
            .runtimes
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?;
        runtimes.retain(|id, _| active_ids.contains(id));

        let mut providers = Vec::new();
        for codec in managed {
            let id = codec.manifest.id.clone();
            let version = codec.manifest.version.clone();
            if let Some(cached) = runtimes.get(&id)
                && cached.version == version
            {
                providers.push(Arc::clone(&cached.provider));
                continue;
            }
            let Ok(registration) =
                AudioCodecRegistration::from_managed_package(codec.manifest, codec.root)
            else {
                runtimes.remove(&id);
                continue;
            };
            let Ok(provider) = ProcessAudioCodecProvider::new(registration) else {
                runtimes.remove(&id);
                continue;
            };
            let provider: Arc<dyn AudioCodecProvider> = Arc::new(provider);
            runtimes.insert(
                id,
                CachedAudioCodec {
                    version,
                    provider: Arc::clone(&provider),
                },
            );
            providers.push(provider);
        }
        Ok(providers)
    }

    fn invalidate(&self, provider_id: &str) -> Result<(), AudioCodecPackageError> {
        self.inner
            .runtimes
            .lock()
            .map_err(|_| AudioCodecPackageError::StateUnavailable)?
            .remove(provider_id);
        Ok(())
    }
}

fn map_package_error(error: ExtensionPackageManagementError) -> AudioCodecPackageError {
    match error {
        ExtensionPackageManagementError::InvalidPackage(_) => {
            AudioCodecPackageError::InvalidPackage
        }
        ExtensionPackageManagementError::VersionConflict => {
            AudioCodecPackageError::PackageVersionConflict
        }
        ExtensionPackageManagementError::RollbackUnavailable => {
            AudioCodecPackageError::RollbackUnavailable
        }
        ExtensionPackageManagementError::RootUnavailable
        | ExtensionPackageManagementError::StorageUnavailable
        | ExtensionPackageManagementError::StateUnavailable
        | ExtensionPackageManagementError::FileOperation => {
            AudioCodecPackageError::PackageStorageUnavailable
        }
    }
}

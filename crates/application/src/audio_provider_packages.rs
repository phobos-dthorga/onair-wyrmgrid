use crate::{
    AudioCaptureProvider, AudioProviderPackageInspection, AudioProviderRegistration,
    ExtensionPackageManagementError, ExtensionPackageService, ExternalAudioProviderProcess,
    ManagedAudioProviderPackageView, inspect_audio_provider_package,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use wyrmgrid_storage::{AudioProviderPreferencesRecord, Store};

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioProviderPackageError {
    #[error("The selected audio provider package is invalid or unsupported.")]
    InvalidPackage,
    #[error("Local audio provider package storage is unavailable.")]
    PackageStorageUnavailable,
    #[error("That audio provider version already exists with different package contents.")]
    PackageVersionConflict,
    #[error("No previous audio provider version is available for rollback.")]
    RollbackUnavailable,
    #[error("That audio provider is not installed or enabled.")]
    UnknownProvider,
    #[error("That audio provider is unavailable on this platform.")]
    ProviderUnavailable,
    #[error("Audio provider selection is unavailable.")]
    SelectionUnavailable,
}

#[derive(Clone)]
pub struct AudioProviderPackageService {
    inner: Arc<AudioProviderPackageServiceInner>,
}

struct AudioProviderPackageServiceInner {
    packages: ExtensionPackageService,
    store: Store,
    runtime: Mutex<Option<CachedAudioProvider>>,
    mutation: Mutex<()>,
}

struct CachedAudioProvider {
    id: String,
    version: String,
    provider: Arc<dyn AudioCaptureProvider>,
}

impl AudioProviderPackageService {
    pub fn new(packages: ExtensionPackageService, store: Store) -> Self {
        Self {
            inner: Arc::new(AudioProviderPackageServiceInner {
                packages,
                store,
                runtime: Mutex::new(None),
                mutation: Mutex::new(()),
            }),
        }
    }

    pub fn inspect_package(
        &self,
        path: &Path,
    ) -> Result<AudioProviderPackageInspection, AudioProviderPackageError> {
        inspect_audio_provider_package(path).map_err(|_| AudioProviderPackageError::InvalidPackage)
    }

    pub fn list_packages(
        &self,
    ) -> Result<Vec<ManagedAudioProviderPackageView>, AudioProviderPackageError> {
        self.inner
            .packages
            .list_audio_provider_packages()
            .map_err(map_package_error)
    }

    pub fn selected_provider_id(&self) -> Result<Option<String>, AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        self.selected_provider_id_unlocked()
    }

    pub fn install_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioProviderPackageView, AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let installed = self
            .inner
            .packages
            .install_audio_provider_package(path)
            .map_err(map_package_error)?;
        self.invalidate_if(&installed.id)?;
        Ok(installed)
    }

    pub fn select_provider(&self, provider_id: &str) -> Result<(), AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let provider = self.build_provider(provider_id)?;
        self.inner
            .store
            .save_audio_provider_preferences_record(&AudioProviderPreferencesRecord {
                selected_provider_id: Some(provider_id.to_owned()),
            })
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        *self
            .inner
            .runtime
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)? = Some(provider);
        Ok(())
    }

    pub fn set_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioProviderPackageView, AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let selected = self.selected_provider_id_unlocked()?;
        let clear_selection = !enabled && selected.as_deref() == Some(provider_id);
        if clear_selection {
            self.save_selection(None)?;
        }
        let result = self
            .inner
            .packages
            .set_audio_provider_enabled(provider_id, enabled)
            .map_err(map_package_error);
        if result.is_err() && clear_selection {
            let _ = self.save_selection(Some(provider_id));
        }
        let view = result?;
        self.invalidate_if(provider_id)?;
        Ok(view)
    }

    pub fn rollback(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioProviderPackageView, AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let view = self
            .inner
            .packages
            .rollback_audio_provider(provider_id)
            .map_err(map_package_error)?;
        self.invalidate_if(provider_id)?;
        Ok(view)
    }

    pub fn remove(&self, provider_id: &str) -> Result<(), AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let selected = self.selected_provider_id_unlocked()?;
        let clear_selection = selected.as_deref() == Some(provider_id);
        if clear_selection {
            self.save_selection(None)?;
        }
        let result = self
            .inner
            .packages
            .remove_audio_provider(provider_id)
            .map_err(map_package_error);
        if result.is_err() && clear_selection {
            let _ = self.save_selection(Some(provider_id));
        }
        result?;
        self.invalidate_if(provider_id)
    }

    pub(crate) fn provider(
        &self,
    ) -> Result<Option<Arc<dyn AudioCaptureProvider>>, AudioProviderPackageError> {
        let _guard = self
            .inner
            .mutation
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        let Some(provider_id) = self.selected_provider_id_unlocked()? else {
            return Ok(None);
        };
        let active_version = self.active_package(&provider_id)?.active_version;
        let mut runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        if let Some(cached) = runtime.as_ref()
            && cached.id == provider_id
            && cached.version == active_version
        {
            return Ok(Some(Arc::clone(&cached.provider)));
        }
        let cached = self.build_provider(&provider_id)?;
        let provider = Arc::clone(&cached.provider);
        *runtime = Some(cached);
        Ok(Some(provider))
    }

    fn selected_provider_id_unlocked(&self) -> Result<Option<String>, AudioProviderPackageError> {
        let selected = self
            .inner
            .store
            .load_audio_provider_preferences_record()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?
            .and_then(|preferences| preferences.selected_provider_id);
        let Some(selected) = selected else {
            return Ok(None);
        };
        Ok(self
            .list_packages()?
            .into_iter()
            .any(|package| package.id == selected && package.enabled)
            .then_some(selected))
    }

    fn active_package(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioProviderPackageView, AudioProviderPackageError> {
        self.list_packages()?
            .into_iter()
            .find(|package| package.id == provider_id && package.enabled)
            .ok_or(AudioProviderPackageError::UnknownProvider)
    }

    fn build_provider(
        &self,
        provider_id: &str,
    ) -> Result<CachedAudioProvider, AudioProviderPackageError> {
        let package = self.active_package(provider_id)?;
        let managed = self
            .inner
            .packages
            .active_managed_audio_providers()
            .map_err(map_package_error)?
            .into_iter()
            .find(|provider| provider.manifest.id == provider_id)
            .ok_or(AudioProviderPackageError::UnknownProvider)?;
        let registration =
            AudioProviderRegistration::from_managed_package(managed.manifest, managed.root)
                .map_err(|_| AudioProviderPackageError::ProviderUnavailable)?;
        let provider = ExternalAudioProviderProcess::new(registration)
            .map_err(|_| AudioProviderPackageError::ProviderUnavailable)?;
        Ok(CachedAudioProvider {
            id: provider_id.to_owned(),
            version: package.active_version,
            provider: Arc::new(provider),
        })
    }

    fn save_selection(&self, provider_id: Option<&str>) -> Result<(), AudioProviderPackageError> {
        self.inner
            .store
            .save_audio_provider_preferences_record(&AudioProviderPreferencesRecord {
                selected_provider_id: provider_id.map(str::to_owned),
            })
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)
    }

    fn invalidate_if(&self, provider_id: &str) -> Result<(), AudioProviderPackageError> {
        let mut runtime = self
            .inner
            .runtime
            .lock()
            .map_err(|_| AudioProviderPackageError::SelectionUnavailable)?;
        if runtime
            .as_ref()
            .is_some_and(|provider| provider.id == provider_id)
        {
            *runtime = None;
        }
        Ok(())
    }
}

fn map_package_error(error: ExtensionPackageManagementError) -> AudioProviderPackageError {
    match error {
        ExtensionPackageManagementError::InvalidPackage(_) => {
            AudioProviderPackageError::InvalidPackage
        }
        ExtensionPackageManagementError::VersionConflict => {
            AudioProviderPackageError::PackageVersionConflict
        }
        ExtensionPackageManagementError::RollbackUnavailable => {
            AudioProviderPackageError::RollbackUnavailable
        }
        ExtensionPackageManagementError::RootUnavailable
        | ExtensionPackageManagementError::StorageUnavailable
        | ExtensionPackageManagementError::StateUnavailable
        | ExtensionPackageManagementError::FileOperation => {
            AudioProviderPackageError::PackageStorageUnavailable
        }
    }
}

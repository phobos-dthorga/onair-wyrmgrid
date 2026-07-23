use chrono::{Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;
use wyrmgrid_audio_provider_protocol::{AudioCaptureEventKind, AudioTrackRequest};
use wyrmgrid_domain::{
    AudioPermissionState, AudioProfileId, AudioSourceAvailability, AudioSourceCapability,
    AudioSourceRole,
};
use wyrmgrid_storage::{
    AudioCaptureEventRecord, AudioRecordingPreferencesRecord, AudioSegmentRecord,
    AudioSessionRecord, AudioSourceSelectionRecord, AudioTrackRecord, Store,
};

use crate::{
    AudioCaptureProvider, AudioCodecError, AudioCodecPackageError, AudioCodecPackageInspection,
    AudioCodecPackageService, AudioCodecProvider, AudioMediaError, AudioProviderError,
    AudioProviderLevel, AudioProviderPackageError, AudioProviderPackageInspection,
    AudioProviderPackageService, AudioSegmentContext, EncodedAudioPacket, EncryptedAudioMediaStore,
    ManagedAudioCodecPackageView, ManagedAudioProviderPackageView, encode_packet_export,
    source_truth_id,
};

pub const DEFAULT_AUDIO_RETENTION_DAYS: u32 = 30;
pub const DEFAULT_AUDIO_STORAGE_BUDGET_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const MAX_PLAYBACK_WINDOW_BYTES: usize = 2 * 1024 * 1024;
const MAX_CAPTURE_DRAIN_FRAMES: u16 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioRecordingPreferences {
    pub enabled: bool,
    pub capture_manual: bool,
    pub capture_automatic: bool,
    pub retention_days: u32,
    pub storage_budget_bytes: u64,
}

impl Default for AudioRecordingPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            capture_manual: false,
            capture_automatic: false,
            retention_days: DEFAULT_AUDIO_RETENTION_DAYS,
            storage_budget_bytes: DEFAULT_AUDIO_STORAGE_BUDGET_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioSourceSelection {
    pub provider_id: String,
    pub source_id: String,
    pub profile_id: AudioProfileId,
    pub codec_provider_id: String,
    pub enabled: bool,
    pub playback_muted: bool,
    pub playback_solo: bool,
    pub playback_volume_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioSourceView {
    pub id: String,
    pub display_name: String,
    pub role: AudioSourceRole,
    pub availability: AudioSourceAvailability,
    pub permission: AudioPermissionState,
    pub supported_profiles: Vec<AudioProfileId>,
    pub codec_provider_id: Option<String>,
    pub enabled: bool,
    pub playback_muted: bool,
    pub playback_solo: bool,
    pub playback_volume_percent: u16,
    pub peak_millidbfs: Option<i32>,
    pub clipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioCodecView {
    pub id: String,
    pub name: String,
    pub supported_profiles: Vec<AudioProfileId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioSessionSummary {
    pub id: String,
    pub simulator_session_id: Option<String>,
    pub provider_id: String,
    pub capture_mode: AudioCaptureMode,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: AudioSessionStatus,
    pub media_availability: AudioMediaAvailability,
    pub total_media_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCaptureMode {
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSessionStatus {
    Active,
    Completed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioMediaAvailability {
    Available,
    NotInBackup,
    Missing,
    Tombstoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioRecordingView {
    pub preferences: AudioRecordingPreferences,
    pub provider_id: Option<String>,
    pub provider_available: bool,
    pub recording_active: bool,
    pub active_session_id: Option<String>,
    pub sources: Vec<AudioSourceView>,
    pub codecs: Vec<AudioCodecView>,
    pub sessions: Vec<AudioSessionSummary>,
    pub last_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioTrackPlaybackView {
    pub track_id: String,
    pub source_id: String,
    pub profile_id: AudioProfileId,
    pub codec_provider_id: String,
    pub codec_provider_version: String,
    pub codec_id: String,
    pub codec_media_type: String,
    pub playback_muted: bool,
    pub playback_solo: bool,
    pub playback_volume_percent: u16,
    pub frame_count: u64,
    pub packets: Vec<EncodedAudioPacketView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EncodedAudioPacketView {
    pub sequence: String,
    pub provider_monotonic_ns: String,
    pub duration_48khz_frames: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioPlaybackView {
    pub session_id: String,
    pub authenticated: bool,
    pub tracks: Vec<AudioTrackPlaybackView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioExportView {
    pub filename: String,
    pub media_type: String,
    pub plaintext_warning_required: bool,
    pub packet_count: u64,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AudioRecordingError {
    #[error("Audio recording consent is disabled.")]
    ConsentDisabled,
    #[error("Audio recording is disabled for this capture mode.")]
    CaptureModeDisabled,
    #[error("No audio source has been explicitly enabled.")]
    NoSourcesSelected,
    #[error("The selected audio source needs an explicit operating-system permission decision.")]
    PermissionRequired,
    #[error("The selected audio source is unavailable.")]
    SourceUnavailable,
    #[error("No audio provider is available.")]
    ProviderUnavailable,
    #[error("The selected audio codec is unavailable or does not support this profile.")]
    CodecUnavailable,
    #[error("Audio recording is already active.")]
    AlreadyRecording,
    #[error("Audio recording is not active.")]
    NotRecording,
    #[error("The selected audio session is still recording.")]
    SessionActive,
    #[error("The selected audio recording is unavailable.")]
    UnknownSession,
    #[error("The selected audio track is unavailable.")]
    UnknownTrack,
    #[error("The stored audio consent or metadata is invalid; recording remains disabled.")]
    InvalidStoredState,
    #[error("The audio preference is outside supported bounds.")]
    InvalidPreference,
    #[error("The authenticated playback window is too large.")]
    PlaybackTooLarge,
    #[error("The export destination already exists.")]
    ExportDestinationExists,
    #[error("Encrypted audio media is unavailable or failed authentication.")]
    MediaUnavailable,
    #[error("Audio metadata storage is unavailable.")]
    StorageUnavailable,
    #[error("Audio recording state is unavailable.")]
    StateUnavailable,
    #[error("The audio provider violated its contract or became unavailable.")]
    ProviderFailed,
    #[error("The audio codec violated its contract or became unavailable.")]
    CodecFailed,
    #[error("The selected audio provider package is invalid or unsupported.")]
    InvalidProviderPackage,
    #[error("Local audio provider package storage is unavailable.")]
    ProviderPackageStorageUnavailable,
    #[error("That audio provider version already exists with different package contents.")]
    ProviderPackageVersionConflict,
    #[error("Stop audio recording before changing the selected provider package.")]
    ProviderPackageInUse,
    #[error("No previous audio provider version is available for rollback.")]
    ProviderRollbackUnavailable,
    #[error("That audio provider is not installed, enabled, or available on this platform.")]
    UnknownProvider,
    #[error("The selected audio codec package is invalid or unsupported.")]
    InvalidCodecPackage,
    #[error("Local audio codec package storage is unavailable.")]
    CodecPackageStorageUnavailable,
    #[error("That audio codec version already exists with different package contents.")]
    CodecPackageVersionConflict,
    #[error("Stop audio recording before changing an audio codec package.")]
    CodecPackageInUse,
    #[error("No previous audio codec version is available for rollback.")]
    CodecRollbackUnavailable,
}

#[derive(Clone)]
pub struct AudioRecordingService {
    inner: Arc<AudioRecordingInner>,
}

struct AudioRecordingInner {
    store: Store,
    media: EncryptedAudioMediaStore,
    provider: AudioProviderAccess,
    codecs: AudioCodecAccess,
    state: Mutex<AudioRuntimeState>,
    operation: Mutex<()>,
}

enum AudioProviderAccess {
    Static(Option<Arc<dyn AudioCaptureProvider>>),
    Managed(AudioProviderPackageService),
}

enum AudioCodecAccess {
    Static(BTreeMap<String, Arc<dyn AudioCodecProvider>>),
    Managed(AudioCodecPackageService),
}

#[derive(Default)]
struct AudioRuntimeState {
    sources: Vec<AudioSourceCapability>,
    active_session_id: Option<String>,
    track_sources: BTreeMap<String, String>,
    track_codecs: BTreeMap<String, String>,
    levels: BTreeMap<String, AudioProviderLevel>,
    last_code: Option<String>,
    reset_in_progress: bool,
}

struct EncodedTrackPacket {
    track_id: String,
    packet: EncodedAudioPacket,
}

fn index_codecs(
    codecs: Vec<Arc<dyn AudioCodecProvider>>,
) -> BTreeMap<String, Arc<dyn AudioCodecProvider>> {
    let mut indexed = BTreeMap::new();
    let mut duplicate_ids = BTreeSet::new();
    for codec in codecs {
        let provider_id = codec.provider_id().to_owned();
        if duplicate_ids.contains(&provider_id) {
            continue;
        }
        if indexed.remove(&provider_id).is_some() {
            duplicate_ids.insert(provider_id);
        } else {
            indexed.insert(provider_id, codec);
        }
    }
    indexed
}

impl AudioRecordingService {
    pub fn new(
        store: Store,
        media: EncryptedAudioMediaStore,
        provider: Option<Arc<dyn AudioCaptureProvider>>,
        codecs: Vec<Arc<dyn AudioCodecProvider>>,
    ) -> Self {
        Self {
            inner: Arc::new(AudioRecordingInner {
                store,
                media,
                provider: AudioProviderAccess::Static(provider),
                codecs: AudioCodecAccess::Static(index_codecs(codecs)),
                state: Mutex::new(AudioRuntimeState::default()),
                operation: Mutex::new(()),
            }),
        }
    }

    pub fn with_managed_provider_packages(
        store: Store,
        media: EncryptedAudioMediaStore,
        provider_packages: AudioProviderPackageService,
        codecs: Vec<Arc<dyn AudioCodecProvider>>,
    ) -> Self {
        Self {
            inner: Arc::new(AudioRecordingInner {
                store,
                media,
                provider: AudioProviderAccess::Managed(provider_packages),
                codecs: AudioCodecAccess::Static(index_codecs(codecs)),
                state: Mutex::new(AudioRuntimeState::default()),
                operation: Mutex::new(()),
            }),
        }
    }

    pub fn with_managed_packages(
        store: Store,
        media: EncryptedAudioMediaStore,
        provider_packages: AudioProviderPackageService,
        codec_packages: AudioCodecPackageService,
    ) -> Self {
        Self {
            inner: Arc::new(AudioRecordingInner {
                store,
                media,
                provider: AudioProviderAccess::Managed(provider_packages),
                codecs: AudioCodecAccess::Managed(codec_packages),
                state: Mutex::new(AudioRuntimeState::default()),
                operation: Mutex::new(()),
            }),
        }
    }

    pub fn status(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        let preferences = self.load_preferences()?;
        let selections = self.load_selections()?;
        let provider = self.provider_optional()?;
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        let sources = state
            .sources
            .iter()
            .map(|source| source_view(source, &selections, &state.track_sources, &state.levels))
            .collect();
        let sessions = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .filter(|record| record.media_availability != "tombstoned")
            .map(session_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let codecs = self
            .codecs()?
            .values()
            .map(|codec| AudioCodecView {
                id: codec.provider_id().into(),
                name: codec.display_name().into(),
                supported_profiles: codec.profiles().iter().map(|profile| profile.id).collect(),
            })
            .collect();
        Ok(AudioRecordingView {
            preferences,
            provider_id: provider
                .as_ref()
                .map(|provider| provider.provider_id().to_owned()),
            provider_available: provider.is_some(),
            recording_active: state.active_session_id.is_some(),
            active_session_id: state.active_session_id.clone(),
            sources,
            codecs,
            sessions,
            last_code: state.last_code.clone(),
        })
    }

    pub fn inspect_provider_package(
        &self,
        path: &Path,
    ) -> Result<AudioProviderPackageInspection, AudioRecordingError> {
        self.managed_provider_packages()?
            .inspect_package(path)
            .map_err(map_provider_package_error)
    }

    pub fn list_managed_provider_packages(
        &self,
    ) -> Result<Vec<ManagedAudioProviderPackageView>, AudioRecordingError> {
        self.managed_provider_packages()?
            .list_packages()
            .map_err(map_provider_package_error)
    }

    pub fn install_provider_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioProviderPackageView, AudioRecordingError> {
        let _operation = self.package_mutation_guard()?;
        let packages = self.managed_provider_packages()?;
        let selected = packages
            .selected_provider_id()
            .map_err(map_provider_package_error)?;
        let installed = packages
            .install_package(path)
            .map_err(map_provider_package_error)?;
        if selected.as_deref() == Some(&installed.id) {
            self.clear_provider_runtime_state()?;
        }
        Ok(installed)
    }

    pub fn select_managed_provider(
        &self,
        provider_id: &str,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        let _operation = self.package_mutation_guard()?;
        self.managed_provider_packages()?
            .select_provider(provider_id)
            .map_err(map_provider_package_error)?;
        self.clear_provider_runtime_state()?;
        self.status()
    }

    pub fn set_managed_provider_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioProviderPackageView, AudioRecordingError> {
        let _operation = self.package_mutation_guard()?;
        let packages = self.managed_provider_packages()?;
        let selected = packages
            .selected_provider_id()
            .map_err(map_provider_package_error)?;
        let view = packages
            .set_enabled(provider_id, enabled)
            .map_err(map_provider_package_error)?;
        if selected.as_deref() == Some(provider_id) {
            self.clear_provider_runtime_state()?;
        }
        Ok(view)
    }

    pub fn rollback_managed_provider(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioProviderPackageView, AudioRecordingError> {
        let _operation = self.package_mutation_guard()?;
        let packages = self.managed_provider_packages()?;
        let selected = packages
            .selected_provider_id()
            .map_err(map_provider_package_error)?;
        let view = packages
            .rollback(provider_id)
            .map_err(map_provider_package_error)?;
        if selected.as_deref() == Some(provider_id) {
            self.clear_provider_runtime_state()?;
        }
        Ok(view)
    }

    pub fn remove_managed_provider(&self, provider_id: &str) -> Result<(), AudioRecordingError> {
        let _operation = self.package_mutation_guard()?;
        let packages = self.managed_provider_packages()?;
        let selected = packages
            .selected_provider_id()
            .map_err(map_provider_package_error)?;
        packages
            .remove(provider_id)
            .map_err(map_provider_package_error)?;
        if selected.as_deref() == Some(provider_id) {
            self.clear_provider_runtime_state()?;
        }
        Ok(())
    }

    pub fn inspect_codec_package(
        &self,
        path: &Path,
    ) -> Result<AudioCodecPackageInspection, AudioRecordingError> {
        self.managed_codec_packages()?
            .inspect_package(path)
            .map_err(map_codec_package_error)
    }

    pub fn list_managed_codec_packages(
        &self,
    ) -> Result<Vec<ManagedAudioCodecPackageView>, AudioRecordingError> {
        self.managed_codec_packages()?
            .list_packages()
            .map_err(map_codec_package_error)
    }

    pub fn install_codec_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, AudioRecordingError> {
        let _operation = self.codec_package_mutation_guard()?;
        self.managed_codec_packages()?
            .install_package(path)
            .map_err(map_codec_package_error)
    }

    pub fn seed_first_party_codec_package(
        &self,
        path: &Path,
    ) -> Result<ManagedAudioCodecPackageView, AudioRecordingError> {
        let _operation = self.codec_package_mutation_guard()?;
        self.managed_codec_packages()?
            .seed_first_party_package(path)
            .map_err(map_codec_package_error)
    }

    pub fn set_managed_codec_enabled(
        &self,
        provider_id: &str,
        enabled: bool,
    ) -> Result<ManagedAudioCodecPackageView, AudioRecordingError> {
        let _operation = self.codec_package_mutation_guard()?;
        self.managed_codec_packages()?
            .set_enabled(provider_id, enabled)
            .map_err(map_codec_package_error)
    }

    pub fn rollback_managed_codec(
        &self,
        provider_id: &str,
    ) -> Result<ManagedAudioCodecPackageView, AudioRecordingError> {
        let _operation = self.codec_package_mutation_guard()?;
        self.managed_codec_packages()?
            .rollback(provider_id)
            .map_err(map_codec_package_error)
    }

    pub fn remove_managed_codec(&self, provider_id: &str) -> Result<(), AudioRecordingError> {
        let _operation = self.codec_package_mutation_guard()?;
        self.managed_codec_packages()?
            .remove(provider_id)
            .map_err(map_codec_package_error)
    }

    pub fn recover_interrupted_sessions(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        self.inner
            .store
            .interrupt_active_audio_session_records(&timestamp())
            .map_err(map_storage_error)?;
        let mut recovery_incomplete = self.inner.media.discard_pending_segments().is_err();
        let retained_storage_keys = self.known_storage_keys()?;
        recovery_incomplete |= self
            .inner
            .media
            .discard_orphan_segments(&retained_storage_keys)
            .is_err();
        let tombstoned_session_ids = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .filter(|session| session.media_availability == "tombstoned")
            .map(|session| session.id)
            .collect::<Vec<_>>();
        for session_id in tombstoned_session_ids {
            if self.delete_session(&session_id).is_err() {
                recovery_incomplete = true;
            }
        }
        if recovery_incomplete {
            self.set_last_code("audio.recovery_incomplete")?;
        }
        self.status()
    }

    pub fn erase_all_media_for_local_reset(&self) -> Result<(), AudioRecordingError> {
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        self.inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?
            .reset_in_progress = true;
        if self.active_session_id()?.is_some() {
            self.stop_locked()?;
        }
        for storage_key in self.known_storage_keys()? {
            self.inner
                .media
                .delete_segment(&storage_key)
                .map_err(map_media_error)?;
        }
        self.inner
            .media
            .discard_orphan_segments(&BTreeSet::new())
            .map_err(map_media_error)?;
        Ok(())
    }

    pub fn update_preferences(
        &self,
        preferences: AudioRecordingPreferences,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        validate_preferences(&preferences)?;
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        if let Some(active_session_id) = self.active_session_id()? {
            let active_mode = self
                .inner
                .store
                .list_audio_session_records()
                .map_err(map_storage_error)?
                .into_iter()
                .find(|session| session.id == active_session_id)
                .map(|session| session.capture_mode)
                .ok_or(AudioRecordingError::InvalidStoredState)?;
            let active_mode_disabled = (active_mode == "manual" && !preferences.capture_manual)
                || (active_mode == "automatic" && !preferences.capture_automatic);
            if !preferences.enabled || active_mode_disabled {
                self.stop_locked()?;
            }
        }
        self.inner
            .store
            .save_audio_recording_preferences_record(&AudioRecordingPreferencesRecord {
                enabled: preferences.enabled,
                capture_manual: preferences.capture_manual,
                capture_automatic: preferences.capture_automatic,
                retention_days: preferences.retention_days,
                storage_budget_bytes: preferences.storage_budget_bytes,
            })
            .map_err(map_storage_error)?;
        self.status()
    }

    pub fn refresh_sources(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        if !self.load_preferences()?.enabled {
            return Err(AudioRecordingError::ConsentDisabled);
        }
        let provider = self.provider()?;
        let sources = provider.sources().map_err(map_provider_error)?;
        validate_source_list(&sources)?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        state.sources = sources;
        state.last_code = Some("audio.sources_refreshed".into());
        drop(state);
        self.status()
    }

    pub fn request_source_permission(
        &self,
        source_id: &str,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        if !self.load_preferences()?.enabled {
            return Err(AudioRecordingError::ConsentDisabled);
        }
        let provider = self.provider()?;
        let sources = provider
            .request_permission(source_id)
            .map_err(map_provider_error)?;
        validate_source_list(&sources)?;
        if !sources.iter().any(|source| source.id == source_id) {
            return Err(AudioRecordingError::ProviderFailed);
        }
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        state.sources = sources;
        state.last_code = Some("audio.permission_updated".into());
        drop(state);
        self.status()
    }

    pub fn update_source_selection(
        &self,
        selection: AudioSourceSelection,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        let provider = self.provider()?;
        if selection.provider_id != provider.provider_id()
            || selection.playback_volume_percent > 200
        {
            return Err(AudioRecordingError::InvalidPreference);
        }
        let codecs = self.codecs()?;
        let codec = codecs
            .get(&selection.codec_provider_id)
            .ok_or(AudioRecordingError::CodecUnavailable)?;
        if !codec
            .profiles()
            .iter()
            .any(|profile| profile.id == selection.profile_id)
        {
            return Err(AudioRecordingError::CodecUnavailable);
        }
        if selection.enabled {
            let sources = provider.sources().map_err(map_provider_error)?;
            validate_source_list(&sources)?;
            let source = sources
                .iter()
                .find(|source| source.id == selection.source_id)
                .ok_or(AudioRecordingError::SourceUnavailable)?;
            if source.availability != AudioSourceAvailability::Available
                || !source.supported_profiles.contains(&selection.profile_id)
            {
                return Err(AudioRecordingError::SourceUnavailable);
            }
        }
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        let current = self.load_selections()?.into_iter().find(|current| {
            current.provider_id == selection.provider_id && current.source_id == selection.source_id
        });
        let active_source = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?
            .track_sources
            .values()
            .any(|source_id| source_id == &selection.source_id);
        let recording_choice_changed = !selection.enabled
            || current.is_some_and(|current| {
                current.profile_id != selection.profile_id
                    || current.codec_provider_id != selection.codec_provider_id
            });
        if active_source && recording_choice_changed {
            self.stop_locked()?;
        }
        self.inner
            .store
            .save_audio_source_selection_record(&AudioSourceSelectionRecord {
                provider_id: selection.provider_id,
                source_id: selection.source_id,
                profile_id: profile_id(selection.profile_id).into(),
                codec_provider_id: selection.codec_provider_id,
                enabled: selection.enabled,
                playback_muted: selection.playback_muted,
                playback_solo: selection.playback_solo,
                playback_volume_percent: selection.playback_volume_percent,
            })
            .map_err(map_storage_error)?;
        self.status()
    }

    pub fn start(
        &self,
        simulator_session_id: Option<String>,
        capture_mode: AudioCaptureMode,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        {
            let state = self
                .inner
                .state
                .lock()
                .map_err(|_| AudioRecordingError::StateUnavailable)?;
            if state.reset_in_progress {
                return Err(AudioRecordingError::ConsentDisabled);
            }
            if state.active_session_id.is_some() {
                return Err(AudioRecordingError::AlreadyRecording);
            }
        }
        let preferences = self.load_preferences()?;
        if !preferences.enabled {
            return Err(AudioRecordingError::ConsentDisabled);
        }
        if (capture_mode == AudioCaptureMode::Manual && !preferences.capture_manual)
            || (capture_mode == AudioCaptureMode::Automatic && !preferences.capture_automatic)
        {
            return Err(AudioRecordingError::CaptureModeDisabled);
        }
        let provider = self.provider()?;
        let sources = provider.sources().map_err(map_provider_error)?;
        validate_source_list(&sources)?;
        let selections = self.load_selections()?;
        let codecs = self.codecs()?;
        let selected = selections
            .iter()
            .filter(|selection| {
                selection.enabled && selection.provider_id == provider.provider_id()
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(AudioRecordingError::NoSourcesSelected);
        }
        let mut requests = Vec::with_capacity(selected.len());
        let mut track_codecs = BTreeMap::new();
        for selection in selected {
            let source = sources
                .iter()
                .find(|source| source.id == selection.source_id)
                .ok_or(AudioRecordingError::SourceUnavailable)?;
            if source.permission == AudioPermissionState::PromptRequired {
                return Err(AudioRecordingError::PermissionRequired);
            }
            if !source.is_capture_ready()
                || !source.supported_profiles.contains(&selection.profile_id)
            {
                return Err(AudioRecordingError::SourceUnavailable);
            }
            let codec = codecs
                .get(&selection.codec_provider_id)
                .ok_or(AudioRecordingError::CodecUnavailable)?;
            if !codec.profiles().iter().any(|profile| {
                profile.id == selection.profile_id
                    && profile.channels == source.channels
                    && profile.sample_rate_hz == 48_000
            }) {
                return Err(AudioRecordingError::CodecUnavailable);
            }
            let track_id = format!("track-{}", Uuid::new_v4().simple());
            track_codecs.insert(track_id.clone(), selection.codec_provider_id.clone());
            requests.push(AudioTrackRequest {
                track_id,
                source_id: source.id.clone(),
                profile: selection.profile_id,
            });
        }

        let session_id = format!("audio-{}", Uuid::new_v4().simple());
        let batch = provider
            .start_capture(&session_id, &requests)
            .map_err(map_provider_error)?;
        if let Err(error) = validate_capture_batch(&session_id, &requests, &batch) {
            let _ = provider.stop_capture(&session_id);
            return Err(error);
        }
        let mut started_codecs = Vec::<(String, String)>::new();
        for request in &requests {
            let codec_id = track_codecs
                .get(&request.track_id)
                .ok_or(AudioRecordingError::CodecUnavailable)?;
            let codec = codecs
                .get(codec_id)
                .ok_or(AudioRecordingError::CodecUnavailable)?;
            if let Err(error) = codec.start_track(&session_id, &request.track_id, request.profile) {
                for (started_codec_id, started_track_id) in &started_codecs {
                    if let Some(started_codec) = codecs.get(started_codec_id) {
                        let _ = started_codec.stop_track(&session_id, started_track_id);
                    }
                }
                let _ = provider.stop_capture(&session_id);
                return Err(map_codec_error(error));
            }
            started_codecs.push((codec_id.clone(), request.track_id.clone()));
        }
        let now = timestamp();
        let tracks = batch
            .tracks
            .iter()
            .map(|started| {
                let source = sources
                    .iter()
                    .find(|source| source.id == started.source_id)
                    .ok_or(AudioRecordingError::ProviderFailed)?;
                let codec_provider_id = track_codecs
                    .get(&started.track_id)
                    .ok_or(AudioRecordingError::CodecUnavailable)?;
                let codec = codecs
                    .get(codec_provider_id)
                    .ok_or(AudioRecordingError::CodecUnavailable)?;
                let codec_profile = codec
                    .profiles()
                    .iter()
                    .find(|profile| profile.id == started.profile)
                    .ok_or(AudioRecordingError::CodecUnavailable)?;
                Ok(AudioTrackRecord {
                    id: started.track_id.clone(),
                    session_id: session_id.clone(),
                    source_id: started.source_id.clone(),
                    profile_id: profile_id(started.profile).into(),
                    codec_provider_id: codec_provider_id.clone(),
                    codec_provider_version: codec.provider_version().into(),
                    codec_id: codec_profile.codec_id.clone(),
                    codec_media_type: codec_profile.media_type.clone(),
                    source_role: source_role_id(source.role).into(),
                    source_truth: source_truth_id(source.truth).into(),
                    channel_count: started.profile.spec().channels,
                    sample_rate_hz: started.profile.spec().sample_rate_hz,
                    provider_start_monotonic_ns: started.provider_start_monotonic_ns,
                    packet_count: 0,
                    frame_count: 0,
                    last_packet_sequence: None,
                })
            })
            .collect::<Result<Vec<_>, AudioRecordingError>>()?;
        let session = AudioSessionRecord {
            id: session_id.clone(),
            simulator_session_id,
            provider_id: provider.provider_id().into(),
            capture_mode: capture_mode_id(capture_mode).into(),
            started_at: now.clone(),
            ended_at: None,
            host_start_monotonic_ns: None,
            status: "active".into(),
            media_availability: "available".into(),
            total_media_bytes: 0,
            deletion_requested_at: None,
        };
        if let Err(error) = self
            .inner
            .store
            .create_audio_session_record(&session, &tracks)
        {
            for (codec_id, track_id) in &started_codecs {
                if let Some(codec) = codecs.get(codec_id) {
                    let _ = codec.stop_track(&session_id, track_id);
                }
            }
            let _ = provider.stop_capture(&session_id);
            return Err(map_storage_error(error));
        }
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        state.sources = sources;
        state.track_sources = tracks
            .iter()
            .map(|track| (track.id.clone(), track.source_id.clone()))
            .collect();
        state.track_codecs = track_codecs;
        state.levels.clear();
        state.active_session_id = Some(session_id);
        state.last_code = Some("audio.capture_active".into());
        drop(state);
        self.status()
    }

    pub fn stop(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        self.stop_locked()
    }

    pub fn poll_active_capture(&self) -> Result<(), AudioRecordingError> {
        let _operation = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        if self.active_session_id()?.is_none() {
            return Ok(());
        }
        if let Err(error) = self.poll_locked() {
            self.interrupt_locked();
            return Err(error);
        }
        Ok(())
    }

    fn stop_locked(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        let session_id = self
            .active_session_id()?
            .ok_or(AudioRecordingError::NotRecording)?;
        let provider = self.provider()?;
        let drained = self
            .poll_locked()
            .is_ok_and(|count| count < usize::from(MAX_CAPTURE_DRAIN_FRAMES));
        let provider_stopped = provider.stop_capture(&session_id).is_ok();
        let codecs_stopped = self.stop_codec_tracks(&session_id);
        let status = if drained && provider_stopped && codecs_stopped {
            "completed"
        } else {
            "interrupted"
        };
        self.inner
            .store
            .finish_audio_session_record(&session_id, &timestamp(), status)
            .map_err(map_storage_error)?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        state.active_session_id = None;
        state.track_sources.clear();
        state.track_codecs.clear();
        state.levels.clear();
        state.last_code = Some(
            if status == "completed" {
                "audio.capture_completed"
            } else {
                "audio.capture_interrupted"
            }
            .into(),
        );
        drop(state);
        let _ = self.enforce_retention();
        self.status()
    }

    fn poll_locked(&self) -> Result<usize, AudioRecordingError> {
        let session_id = self
            .active_session_id()?
            .ok_or(AudioRecordingError::NotRecording)?;
        let batch = self
            .provider()?
            .drain_capture(&session_id, MAX_CAPTURE_DRAIN_FRAMES)
            .map_err(map_provider_error)?;
        let track_codecs = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?
            .track_codecs
            .clone();
        let codecs = self.codecs()?;
        let drained_frame_count = batch.frames.len();
        let mut packets = Vec::with_capacity(drained_frame_count);
        for frame in &batch.frames {
            if frame.session_id != session_id {
                return Err(AudioRecordingError::ProviderFailed);
            }
            let codec_id = track_codecs
                .get(&frame.track_id)
                .ok_or(AudioRecordingError::ProviderFailed)?;
            let codec = codecs
                .get(codec_id)
                .ok_or(AudioRecordingError::CodecUnavailable)?;
            packets.push(EncodedTrackPacket {
                track_id: frame.track_id.clone(),
                packet: codec.encode_pcm(frame).map_err(map_codec_error)?,
            });
        }
        self.persist_batch(&session_id, &packets, &batch.events, &timestamp())?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        for level in batch.levels {
            if !state.track_sources.contains_key(&level.track_id) {
                return Err(AudioRecordingError::ProviderFailed);
            }
            state.levels.insert(level.track_id.clone(), level);
        }
        Ok(drained_frame_count)
    }

    fn stop_codec_tracks(&self, session_id: &str) -> bool {
        let track_codecs = match self.inner.state.lock() {
            Ok(state) => state.track_codecs.clone(),
            Err(_) => return false,
        };
        let Ok(codecs) = self.codecs() else {
            return false;
        };
        track_codecs
            .into_iter()
            .fold(true, |all_stopped, (track_id, codec_id)| {
                let stopped = codecs
                    .get(&codec_id)
                    .is_some_and(|codec| codec.stop_track(session_id, &track_id).is_ok());
                all_stopped && stopped
            })
    }

    fn interrupt_locked(&self) {
        let Ok(Some(session_id)) = self.active_session_id() else {
            return;
        };
        if let Ok(Some(provider)) = self.provider_optional() {
            let _ = provider.stop_capture(&session_id);
        }
        self.stop_codec_tracks(&session_id);
        let _ =
            self.inner
                .store
                .finish_audio_session_record(&session_id, &timestamp(), "interrupted");
        if let Ok(mut state) = self.inner.state.lock() {
            state.active_session_id = None;
            state.track_sources.clear();
            state.track_codecs.clear();
            state.levels.clear();
            state.last_code = Some("audio.capture_interrupted".into());
        }
    }

    pub fn playback(&self, session_id: &str) -> Result<AudioPlaybackView, AudioRecordingError> {
        let session = self.readable_session(session_id)?;
        let selections = self.load_selections()?;
        let tracks = self
            .inner
            .store
            .list_audio_track_records(session_id)
            .map_err(map_storage_error)?;
        let mut total_bytes = 0_usize;
        let mut views = Vec::new();
        for track in tracks {
            let packets = self.read_track_packets(&track)?;
            total_bytes = total_bytes
                .checked_add(
                    packets
                        .iter()
                        .map(|packet| packet.bytes.len())
                        .sum::<usize>(),
                )
                .ok_or(AudioRecordingError::PlaybackTooLarge)?;
            if total_bytes > MAX_PLAYBACK_WINDOW_BYTES {
                return Err(AudioRecordingError::PlaybackTooLarge);
            }
            let selection = selections.iter().find(|selection| {
                selection.provider_id == session.provider_id
                    && selection.source_id == track.source_id
            });
            views.push(AudioTrackPlaybackView {
                track_id: track.id,
                source_id: track.source_id,
                profile_id: parse_profile_id(&track.profile_id)?,
                codec_provider_id: track.codec_provider_id,
                codec_provider_version: track.codec_provider_version,
                codec_id: track.codec_id,
                codec_media_type: track.codec_media_type,
                playback_muted: selection.is_some_and(|selection| selection.playback_muted),
                playback_solo: selection.is_some_and(|selection| selection.playback_solo),
                playback_volume_percent: selection
                    .map_or(100, |selection| selection.playback_volume_percent),
                frame_count: track.frame_count,
                packets: packets.into_iter().map(packet_view).collect(),
            });
        }
        Ok(AudioPlaybackView {
            session_id: session_id.into(),
            authenticated: true,
            tracks: views,
        })
    }

    pub fn export_track(
        &self,
        session_id: &str,
        track_id: &str,
        destination: &Path,
    ) -> Result<AudioExportView, AudioRecordingError> {
        if destination.exists() {
            return Err(AudioRecordingError::ExportDestinationExists);
        }
        self.readable_session(session_id)?;
        let track = self
            .inner
            .store
            .list_audio_track_records(session_id)
            .map_err(map_storage_error)?
            .into_iter()
            .find(|track| track.id == track_id)
            .ok_or(AudioRecordingError::UnknownTrack)?;
        let packets = self.read_track_packets(&track)?;
        let plaintext = encode_packet_export(&packets).map_err(map_media_error)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)
            .map_err(|_| AudioRecordingError::MediaUnavailable)?;
        if let Err(error) = file.write_all(&plaintext).and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(destination);
            return Err(map_media_error(AudioMediaError::Write(error)));
        }
        Ok(AudioExportView {
            filename: destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("audio.wyrmgrid-audio-packets")
                .into(),
            media_type: "application/vnd.wyrmgrid.audio-packets".into(),
            plaintext_warning_required: true,
            packet_count: packets.len() as u64,
        })
    }

    pub fn delete_session(
        &self,
        session_id: &str,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        if self.active_session_id()?.as_deref() == Some(session_id) {
            return Err(AudioRecordingError::SessionActive);
        }
        let session = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or(AudioRecordingError::UnknownSession)?;
        if session.status == "active" {
            return Err(AudioRecordingError::SessionActive);
        }
        let tracks = self
            .inner
            .store
            .list_audio_track_records(session_id)
            .map_err(map_storage_error)?;
        if tracks.is_empty() {
            return Err(AudioRecordingError::UnknownSession);
        }
        let requested_at = timestamp();
        self.inner
            .store
            .mark_audio_session_tombstoned(session_id, &requested_at)
            .map_err(map_storage_error)?;
        for track in &tracks {
            for segment in self
                .inner
                .store
                .list_audio_segment_records(&track.id)
                .map_err(map_storage_error)?
            {
                if self
                    .inner
                    .media
                    .delete_segment(&segment.storage_key)
                    .is_err()
                {
                    self.set_last_code("audio.deletion_pending")?;
                    return Err(AudioRecordingError::MediaUnavailable);
                }
            }
        }
        self.inner
            .store
            .delete_audio_session_metadata(session_id)
            .map_err(map_storage_error)?;
        self.set_last_code("audio.deleted")?;
        self.status()
    }

    pub fn delete_linked_simulator_session(
        &self,
        simulator_session_id: &str,
    ) -> Result<AudioRecordingView, AudioRecordingError> {
        let linked = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .filter(|session| session.simulator_session_id.as_deref() == Some(simulator_session_id))
            .map(|session| session.id)
            .collect::<Vec<_>>();
        for session_id in linked {
            self.delete_session(&session_id)?;
        }
        self.status()
    }

    pub fn enforce_retention(&self) -> Result<AudioRecordingView, AudioRecordingError> {
        let preferences = self.load_preferences()?;
        let all_sessions = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?;
        let mut retained_bytes = all_sessions
            .iter()
            .filter(|session| {
                matches!(
                    session.media_availability.as_str(),
                    "available" | "tombstoned"
                )
            })
            .try_fold(0_u64, |total, session| {
                total.checked_add(session.total_media_bytes)
            })
            .ok_or(AudioRecordingError::InvalidStoredState)?;
        let candidates = self
            .inner
            .store
            .list_audio_deletion_candidate_records()
            .map_err(map_storage_error)?;
        let cutoff = Utc::now() - Duration::days(i64::from(preferences.retention_days));
        for candidate in candidates {
            let reference = candidate
                .ended_at
                .as_deref()
                .unwrap_or(&candidate.started_at);
            let expired = chrono::DateTime::parse_from_rfc3339(reference)
                .map_err(|_| AudioRecordingError::InvalidStoredState)?
                .with_timezone(&Utc)
                < cutoff;
            if expired || retained_bytes > preferences.storage_budget_bytes {
                let bytes = candidate.total_media_bytes;
                let counted = matches!(
                    candidate.media_availability.as_str(),
                    "available" | "tombstoned"
                );
                self.delete_session(&candidate.id)?;
                if counted {
                    retained_bytes = retained_bytes.saturating_sub(bytes);
                }
            }
        }
        self.status()
    }

    pub fn synchronize_with_simulator_recording(
        &self,
        simulator_session_id: Option<String>,
        capture_mode: AudioCaptureMode,
    ) {
        let active = self.active_session_id().ok().flatten();
        match (simulator_session_id, active) {
            (Some(simulator_session_id), None) => {
                if let Err(error) = self.start(Some(simulator_session_id), capture_mode) {
                    let _ = self.set_last_code(error_code(error));
                }
            }
            (None, Some(_)) => {
                let _ = self.stop();
            }
            _ => {}
        }
    }

    fn persist_batch(
        &self,
        session_id: &str,
        packets: &[EncodedTrackPacket],
        events: &[crate::AudioProviderEvent],
        observed_at: &str,
    ) -> Result<(), AudioRecordingError> {
        let tracks = self
            .inner
            .store
            .list_audio_track_records(session_id)
            .map_err(map_storage_error)?;
        for track in &tracks {
            let track_packets = packets
                .iter()
                .filter(|packet| packet.track_id == track.id)
                .map(|packet| packet.packet.clone())
                .collect::<Vec<_>>();
            if track_packets.is_empty() {
                continue;
            }
            if track_packets
                .windows(2)
                .any(|pair| pair[0].sequence >= pair[1].sequence)
                || track
                    .last_packet_sequence
                    .is_some_and(|last| track_packets[0].sequence <= last)
            {
                return Err(AudioRecordingError::CodecFailed);
            }
            let frame_count = track_packets
                .iter()
                .try_fold(0_u64, |total, packet| {
                    total.checked_add(u64::from(packet.duration_48khz_frames))
                })
                .ok_or(AudioRecordingError::ProviderFailed)?;
            let segments = self
                .inner
                .store
                .list_audio_segment_records(&track.id)
                .map_err(map_storage_error)?;
            let segment_index = u32::try_from(segments.len())
                .map_err(|_| AudioRecordingError::StorageUnavailable)?;
            let context = AudioSegmentContext {
                session_id: session_id.into(),
                track_id: track.id.clone(),
                segment_index,
                first_frame: track.frame_count,
                frame_count,
            };
            let stored = self
                .inner
                .media
                .write_segment(&context, &track_packets)
                .map_err(map_media_error)?;
            self.inner
                .store
                .complete_audio_segment_record(
                    &AudioSegmentRecord {
                        track_id: track.id.clone(),
                        segment_index,
                        storage_key: stored.storage_key,
                        first_frame: track.frame_count,
                        frame_count,
                        packet_count: track_packets.len() as u64,
                        encrypted_bytes: stored.encrypted_bytes,
                        envelope_sha256: stored.envelope_sha256,
                        envelope_version: stored.envelope_version,
                        key_version: stored.key_version,
                        state: "complete".into(),
                        created_at: observed_at.into(),
                        deletion_requested_at: None,
                    },
                    track_packets.last().expect("nonempty").sequence,
                )
                .map_err(map_storage_error)?;
        }
        for event in events {
            self.inner
                .store
                .save_audio_capture_event_record(&AudioCaptureEventRecord {
                    session_id: session_id.into(),
                    track_id: event.track_id.clone(),
                    provider_monotonic_ns: event.provider_monotonic_ns,
                    event_kind: event_kind_id(event.event).into(),
                    code: event.code.clone(),
                    affected_frames: event.affected_frames,
                    drift_parts_per_million: event.drift_parts_per_million,
                    observed_at: observed_at.into(),
                })
                .map_err(map_storage_error)?;
        }
        Ok(())
    }

    fn read_track_packets(
        &self,
        track: &AudioTrackRecord,
    ) -> Result<Vec<EncodedAudioPacket>, AudioRecordingError> {
        let segments = self
            .inner
            .store
            .list_audio_segment_records(&track.id)
            .map_err(map_storage_error)?;
        let mut packets = Vec::new();
        for segment in segments {
            if segment.state != "complete" {
                return Err(AudioRecordingError::MediaUnavailable);
            }
            let context = AudioSegmentContext {
                session_id: track.session_id.clone(),
                track_id: track.id.clone(),
                segment_index: segment.segment_index,
                first_frame: segment.first_frame,
                frame_count: segment.frame_count,
            };
            packets.extend(
                self.inner
                    .media
                    .read_segment(&segment.storage_key, &segment.envelope_sha256, &context)
                    .map_err(map_media_error)?,
            );
        }
        if packets.is_empty()
            || packets
                .windows(2)
                .any(|pair| pair[0].sequence >= pair[1].sequence)
        {
            return Err(AudioRecordingError::MediaUnavailable);
        }
        Ok(packets)
    }

    fn readable_session(
        &self,
        session_id: &str,
    ) -> Result<AudioSessionRecord, AudioRecordingError> {
        let session = self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .find(|session| session.id == session_id)
            .ok_or(AudioRecordingError::UnknownSession)?;
        if session.status == "active" {
            return Err(AudioRecordingError::SessionActive);
        }
        if session.media_availability != "available" || session.deletion_requested_at.is_some() {
            return Err(AudioRecordingError::MediaUnavailable);
        }
        Ok(session)
    }

    fn load_preferences(&self) -> Result<AudioRecordingPreferences, AudioRecordingError> {
        self.inner
            .store
            .load_audio_recording_preferences_record()
            .map_err(map_storage_error)?
            .map(|record| AudioRecordingPreferences {
                enabled: record.enabled,
                capture_manual: record.capture_manual,
                capture_automatic: record.capture_automatic,
                retention_days: record.retention_days,
                storage_budget_bytes: record.storage_budget_bytes,
            })
            .map_or_else(
                || Ok(AudioRecordingPreferences::default()),
                |preferences| {
                    validate_preferences(&preferences)?;
                    Ok(preferences)
                },
            )
    }

    fn load_selections(&self) -> Result<Vec<AudioSourceSelection>, AudioRecordingError> {
        self.inner
            .store
            .list_audio_source_selection_records()
            .map_err(map_storage_error)?
            .into_iter()
            .map(|record| {
                Ok(AudioSourceSelection {
                    provider_id: record.provider_id,
                    source_id: record.source_id,
                    profile_id: parse_profile_id(&record.profile_id)?,
                    codec_provider_id: record.codec_provider_id,
                    enabled: record.enabled,
                    playback_muted: record.playback_muted,
                    playback_solo: record.playback_solo,
                    playback_volume_percent: record.playback_volume_percent,
                })
            })
            .collect()
    }

    fn provider(&self) -> Result<Arc<dyn AudioCaptureProvider>, AudioRecordingError> {
        self.provider_optional()?
            .ok_or(AudioRecordingError::ProviderUnavailable)
    }

    fn provider_optional(
        &self,
    ) -> Result<Option<Arc<dyn AudioCaptureProvider>>, AudioRecordingError> {
        match &self.inner.provider {
            AudioProviderAccess::Static(provider) => Ok(provider.clone()),
            AudioProviderAccess::Managed(packages) => {
                packages.provider().map_err(map_provider_package_error)
            }
        }
    }

    fn managed_provider_packages(
        &self,
    ) -> Result<&AudioProviderPackageService, AudioRecordingError> {
        match &self.inner.provider {
            AudioProviderAccess::Managed(packages) => Ok(packages),
            AudioProviderAccess::Static(_) => {
                Err(AudioRecordingError::ProviderPackageStorageUnavailable)
            }
        }
    }

    fn managed_codec_packages(&self) -> Result<&AudioCodecPackageService, AudioRecordingError> {
        match &self.inner.codecs {
            AudioCodecAccess::Managed(packages) => Ok(packages),
            AudioCodecAccess::Static(_) => Err(AudioRecordingError::CodecPackageStorageUnavailable),
        }
    }

    fn codecs(&self) -> Result<BTreeMap<String, Arc<dyn AudioCodecProvider>>, AudioRecordingError> {
        match &self.inner.codecs {
            AudioCodecAccess::Static(codecs) => Ok(codecs.clone()),
            AudioCodecAccess::Managed(packages) => packages
                .providers()
                .map(index_codecs)
                .map_err(map_codec_package_error),
        }
    }

    fn package_mutation_guard(&self) -> Result<std::sync::MutexGuard<'_, ()>, AudioRecordingError> {
        let guard = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        if self.active_session_id()?.is_some() {
            return Err(AudioRecordingError::ProviderPackageInUse);
        }
        Ok(guard)
    }

    fn codec_package_mutation_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ()>, AudioRecordingError> {
        let guard = self
            .inner
            .operation
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        if self.active_session_id()?.is_some() {
            return Err(AudioRecordingError::CodecPackageInUse);
        }
        Ok(guard)
    }

    fn clear_provider_runtime_state(&self) -> Result<(), AudioRecordingError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?;
        state.sources.clear();
        state.track_sources.clear();
        state.levels.clear();
        state.last_code = Some("audio.provider_changed".into());
        Ok(())
    }

    fn active_session_id(&self) -> Result<Option<String>, AudioRecordingError> {
        self.inner
            .state
            .lock()
            .map(|state| state.active_session_id.clone())
            .map_err(|_| AudioRecordingError::StateUnavailable)
    }

    fn set_last_code(&self, code: &str) -> Result<(), AudioRecordingError> {
        self.inner
            .state
            .lock()
            .map_err(|_| AudioRecordingError::StateUnavailable)?
            .last_code = Some(code.into());
        Ok(())
    }

    fn known_storage_keys(&self) -> Result<BTreeSet<String>, AudioRecordingError> {
        let mut keys = BTreeSet::new();
        for session in self
            .inner
            .store
            .list_audio_session_records()
            .map_err(map_storage_error)?
            .into_iter()
            .filter(|session| {
                matches!(
                    session.media_availability.as_str(),
                    "available" | "tombstoned"
                )
            })
        {
            for track in self
                .inner
                .store
                .list_audio_track_records(&session.id)
                .map_err(map_storage_error)?
            {
                for segment in self
                    .inner
                    .store
                    .list_audio_segment_records(&track.id)
                    .map_err(map_storage_error)?
                    .into_iter()
                    .filter(|segment| matches!(segment.state.as_str(), "complete" | "tombstoned"))
                {
                    keys.insert(segment.storage_key);
                }
            }
        }
        Ok(keys)
    }
}

fn source_view(
    source: &AudioSourceCapability,
    selections: &[AudioSourceSelection],
    track_sources: &BTreeMap<String, String>,
    levels: &BTreeMap<String, AudioProviderLevel>,
) -> AudioSourceView {
    let selection = selections
        .iter()
        .find(|selection| selection.source_id == source.id);
    let level = track_sources
        .iter()
        .find(|(_, source_id)| source_id.as_str() == source.id)
        .and_then(|(track_id, _)| levels.get(track_id));
    AudioSourceView {
        id: source.id.clone(),
        display_name: source.display_name.clone(),
        role: source.role,
        availability: source.availability,
        permission: source.permission,
        supported_profiles: source.supported_profiles.clone(),
        codec_provider_id: selection.map(|selection| selection.codec_provider_id.clone()),
        enabled: selection.is_some_and(|selection| selection.enabled),
        playback_muted: selection.is_some_and(|selection| selection.playback_muted),
        playback_solo: selection.is_some_and(|selection| selection.playback_solo),
        playback_volume_percent: selection
            .map_or(100, |selection| selection.playback_volume_percent),
        peak_millidbfs: level.map(|level| level.peak_millidbfs),
        clipped: level.is_some_and(|level| level.clipped),
    }
}

fn validate_source_list(sources: &[AudioSourceCapability]) -> Result<(), AudioRecordingError> {
    let mut ids = BTreeSet::new();
    if sources
        .iter()
        .any(|source| source.validate().is_err() || !ids.insert(source.id.as_str()))
    {
        Err(AudioRecordingError::ProviderFailed)
    } else {
        Ok(())
    }
}

fn validate_capture_batch(
    _session_id: &str,
    requests: &[AudioTrackRequest],
    batch: &crate::AudioProviderCaptureBatch,
) -> Result<(), AudioRecordingError> {
    if batch.tracks.len() != requests.len() {
        return Err(AudioRecordingError::ProviderFailed);
    }
    let requested_tracks = requests
        .iter()
        .map(|request| (request.track_id.as_str(), request))
        .collect::<BTreeMap<_, _>>();
    let started_track_ids = batch
        .tracks
        .iter()
        .map(|track| track.track_id.as_str())
        .collect::<BTreeSet<_>>();
    if requested_tracks.len() != requests.len()
        || started_track_ids.len() != requests.len()
        || batch.tracks.iter().any(|track| {
            requested_tracks
                .get(track.track_id.as_str())
                .is_none_or(|request| {
                    request.source_id != track.source_id || request.profile != track.profile
                })
        })
    {
        return Err(AudioRecordingError::ProviderFailed);
    }

    Ok(())
}

fn session_summary(record: AudioSessionRecord) -> Result<AudioSessionSummary, AudioRecordingError> {
    Ok(AudioSessionSummary {
        id: record.id,
        simulator_session_id: record.simulator_session_id,
        provider_id: record.provider_id,
        capture_mode: match record.capture_mode.as_str() {
            "manual" => AudioCaptureMode::Manual,
            "automatic" => AudioCaptureMode::Automatic,
            _ => return Err(AudioRecordingError::InvalidStoredState),
        },
        started_at: record.started_at,
        ended_at: record.ended_at,
        status: match record.status.as_str() {
            "active" => AudioSessionStatus::Active,
            "completed" => AudioSessionStatus::Completed,
            "interrupted" => AudioSessionStatus::Interrupted,
            _ => return Err(AudioRecordingError::InvalidStoredState),
        },
        media_availability: match record.media_availability.as_str() {
            "available" => AudioMediaAvailability::Available,
            "not_in_backup" => AudioMediaAvailability::NotInBackup,
            "missing" => AudioMediaAvailability::Missing,
            "tombstoned" => AudioMediaAvailability::Tombstoned,
            _ => return Err(AudioRecordingError::InvalidStoredState),
        },
        total_media_bytes: record.total_media_bytes,
    })
}

fn packet_view(packet: EncodedAudioPacket) -> EncodedAudioPacketView {
    EncodedAudioPacketView {
        sequence: packet.sequence.to_string(),
        provider_monotonic_ns: packet.provider_monotonic_ns.to_string(),
        duration_48khz_frames: packet.duration_48khz_frames,
        bytes: packet.bytes,
    }
}

fn validate_preferences(
    preferences: &AudioRecordingPreferences,
) -> Result<(), AudioRecordingError> {
    if !(1..=3650).contains(&preferences.retention_days)
        || !(16 * 1024 * 1024..=1024 * 1024 * 1024 * 1024)
            .contains(&preferences.storage_budget_bytes)
    {
        Err(AudioRecordingError::InvalidPreference)
    } else {
        Ok(())
    }
}

fn profile_id(profile: AudioProfileId) -> &'static str {
    match profile {
        AudioProfileId::PilotMicrophoneV1 => "pilot_microphone_v1",
        AudioProfileId::IsolatedVoiceV1 => "isolated_voice_v1",
        AudioProfileId::MixedStereoV1 => "mixed_stereo_v1",
    }
}

fn parse_profile_id(value: &str) -> Result<AudioProfileId, AudioRecordingError> {
    match value {
        "pilot_microphone_v1" => Ok(AudioProfileId::PilotMicrophoneV1),
        "isolated_voice_v1" => Ok(AudioProfileId::IsolatedVoiceV1),
        "mixed_stereo_v1" => Ok(AudioProfileId::MixedStereoV1),
        _ => Err(AudioRecordingError::InvalidStoredState),
    }
}

fn source_role_id(role: AudioSourceRole) -> &'static str {
    match role {
        AudioSourceRole::MicrophoneInput => "microphone_input",
        AudioSourceRole::ApplicationOutput => "application_output",
        AudioSourceRole::OutputEndpoint => "output_endpoint",
        AudioSourceRole::SimulatorMasterMix => "simulator_master_mix",
        AudioSourceRole::IsolatedCom1 => "isolated_com1",
        AudioSourceRole::IsolatedCom2 => "isolated_com2",
        AudioSourceRole::PilotRadio => "pilot_radio",
        AudioSourceRole::CopilotRadio => "copilot_radio",
    }
}

fn capture_mode_id(mode: AudioCaptureMode) -> &'static str {
    match mode {
        AudioCaptureMode::Manual => "manual",
        AudioCaptureMode::Automatic => "automatic",
    }
}

fn event_kind_id(event: AudioCaptureEventKind) -> &'static str {
    match event {
        AudioCaptureEventKind::PermissionRequired => "permission_required",
        AudioCaptureEventKind::PermissionDenied => "permission_denied",
        AudioCaptureEventKind::SourceUnavailable => "source_unavailable",
        AudioCaptureEventKind::SourceChanged => "source_changed",
        AudioCaptureEventKind::Gap => "gap",
        AudioCaptureEventKind::Dropout => "dropout",
        AudioCaptureEventKind::Drift => "drift",
        AudioCaptureEventKind::Backpressure => "backpressure",
        AudioCaptureEventKind::EncoderFailure => "encoder_failure",
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn map_storage_error(error: wyrmgrid_storage::StorageError) -> AudioRecordingError {
    if matches!(error, wyrmgrid_storage::StorageError::InvalidRecord) {
        AudioRecordingError::InvalidStoredState
    } else {
        AudioRecordingError::StorageUnavailable
    }
}
fn map_provider_error(error: AudioProviderError) -> AudioRecordingError {
    match error {
        AudioProviderError::Unavailable | AudioProviderError::SourceUnavailable => {
            AudioRecordingError::ProviderUnavailable
        }
        _ => AudioRecordingError::ProviderFailed,
    }
}
fn map_provider_package_error(error: AudioProviderPackageError) -> AudioRecordingError {
    match error {
        AudioProviderPackageError::InvalidPackage => AudioRecordingError::InvalidProviderPackage,
        AudioProviderPackageError::PackageStorageUnavailable
        | AudioProviderPackageError::SelectionUnavailable => {
            AudioRecordingError::ProviderPackageStorageUnavailable
        }
        AudioProviderPackageError::PackageVersionConflict => {
            AudioRecordingError::ProviderPackageVersionConflict
        }
        AudioProviderPackageError::RollbackUnavailable => {
            AudioRecordingError::ProviderRollbackUnavailable
        }
        AudioProviderPackageError::UnknownProvider
        | AudioProviderPackageError::ProviderUnavailable => AudioRecordingError::UnknownProvider,
    }
}
fn map_codec_package_error(error: AudioCodecPackageError) -> AudioRecordingError {
    match error {
        AudioCodecPackageError::InvalidPackage => AudioRecordingError::InvalidCodecPackage,
        AudioCodecPackageError::PackageStorageUnavailable
        | AudioCodecPackageError::StateUnavailable => {
            AudioRecordingError::CodecPackageStorageUnavailable
        }
        AudioCodecPackageError::PackageVersionConflict => {
            AudioRecordingError::CodecPackageVersionConflict
        }
        AudioCodecPackageError::RollbackUnavailable => {
            AudioRecordingError::CodecRollbackUnavailable
        }
        AudioCodecPackageError::ProviderUnavailable => AudioRecordingError::CodecUnavailable,
    }
}
fn map_codec_error(error: AudioCodecError) -> AudioRecordingError {
    match error {
        AudioCodecError::Unavailable | AudioCodecError::UnsupportedProfile => {
            AudioRecordingError::CodecUnavailable
        }
        _ => AudioRecordingError::CodecFailed,
    }
}
fn map_media_error(_: AudioMediaError) -> AudioRecordingError {
    AudioRecordingError::MediaUnavailable
}
fn error_code(error: AudioRecordingError) -> &'static str {
    match error {
        AudioRecordingError::ConsentDisabled => "audio.consent_disabled",
        AudioRecordingError::CaptureModeDisabled => "audio.capture_mode_disabled",
        AudioRecordingError::NoSourcesSelected => "audio.no_sources_selected",
        AudioRecordingError::PermissionRequired => "audio.permission_required",
        AudioRecordingError::SourceUnavailable => "audio.source_unavailable",
        AudioRecordingError::ProviderUnavailable => "audio.provider_unavailable",
        AudioRecordingError::ProviderFailed => "audio.provider_failed",
        AudioRecordingError::CodecUnavailable => "audio.codec_unavailable",
        AudioRecordingError::CodecFailed => "audio.codec_failed",
        AudioRecordingError::InvalidStoredState => "audio.invalid_stored_state",
        _ => "audio.capture_unavailable",
    }
}

#[cfg(test)]
#[path = "tests/audio_recording.rs"]
mod tests;

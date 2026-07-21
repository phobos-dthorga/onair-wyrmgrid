//! Versioned, data-only localization packs and local language preferences.
//!
//! English (Australia) is the canonical source catalogue. Community packs may
//! override only ordinary interface messages; security-sensitive and legal
//! namespaces require a reviewed first-party pack.

use fluent_syntax::parser;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;
use thiserror::Error;
use wyrmgrid_storage::Store;

pub const LANGUAGE_PACK_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_CATALOG_VERSION: u32 = 21;
pub const SOURCE_LOCALE: &str = "en-AU";
pub const DEFAULT_LANGUAGE_PACK_ID: &str = "wyrmgrid-en-au";
const MAX_LANGUAGE_PACK_BYTES: usize = 256 * 1024;
const MAX_LANGUAGE_MESSAGES: usize = 2_048;
const MAX_MESSAGE_PATTERN_BYTES: usize = 2_048;
const PROTECTED_MESSAGE_PREFIXES: [&str; 9] = [
    "legal-",
    "privacy-",
    "credential-",
    "telemetry-",
    "plugin-permission-",
    "security-",
    "data-protection-",
    "destructive-",
    "error-",
];

static SOURCE_CATALOG: LazyLock<LanguagePackManifest> = LazyLock::new(|| {
    let manifest: LanguagePackManifest =
        serde_json::from_str(include_str!("../../../locales/en-AU.json"))
            .expect("the built-in English catalogue must be valid JSON");
    validate_manifest(&manifest, PackValidationMode::BuiltIn)
        .expect("the built-in English catalogue must satisfy the language-pack contract");
    manifest
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguagePackManifest {
    pub schema_version: u32,
    pub id: String,
    pub locale: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub source_locale: String,
    pub source_catalog_version: u32,
    pub direction: TextDirection,
    pub messages: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePackTrust {
    BuiltIn,
    Reviewed,
    Community,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AvailableLanguagePack {
    pub manifest: LanguagePackManifest,
    pub trust: LanguagePackTrust,
    pub translated_messages: usize,
    pub eligible_messages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageStatus {
    pub selected_language_pack_id: String,
    pub active_pack: LanguagePackManifest,
    pub packs: Vec<AvailableLanguagePack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCustomLanguagePack {
    pub pack_id: String,
    pub manifest_json: String,
}

pub trait LanguagePackRepository: Send + Sync + 'static {
    fn load_selected_language_pack(&self) -> Result<Option<String>, LanguageSettingsError>;
    fn save_selected_language_pack(&self, pack_id: &str) -> Result<(), LanguageSettingsError>;
    fn list_custom_language_packs(
        &self,
    ) -> Result<Vec<PersistedCustomLanguagePack>, LanguageSettingsError>;
    fn save_custom_language_pack(
        &self,
        pack_id: &str,
        manifest_json: &str,
    ) -> Result<(), LanguageSettingsError>;
}

impl LanguagePackRepository for Store {
    fn load_selected_language_pack(&self) -> Result<Option<String>, LanguageSettingsError> {
        self.load_language_preferences_record()
            .map(|record| record.map(|record| record.selected_language_pack_id))
            .map_err(|_| LanguageSettingsError::StorageUnavailable)
    }

    fn save_selected_language_pack(&self, pack_id: &str) -> Result<(), LanguageSettingsError> {
        self.save_selected_language_pack_record(pack_id)
            .map_err(|_| LanguageSettingsError::StorageUnavailable)
    }

    fn list_custom_language_packs(
        &self,
    ) -> Result<Vec<PersistedCustomLanguagePack>, LanguageSettingsError> {
        self.list_custom_language_pack_records()
            .map(|records| {
                records
                    .into_iter()
                    .map(|record| PersistedCustomLanguagePack {
                        pack_id: record.pack_id,
                        manifest_json: record.manifest_json,
                    })
                    .collect()
            })
            .map_err(|_| LanguageSettingsError::StorageUnavailable)
    }

    fn save_custom_language_pack(
        &self,
        pack_id: &str,
        manifest_json: &str,
    ) -> Result<(), LanguageSettingsError> {
        self.save_custom_language_pack_record(pack_id, manifest_json)
            .map_err(|_| LanguageSettingsError::StorageUnavailable)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSettingsError {
    #[error("WyrmGrid could not read or save its local language settings.")]
    StorageUnavailable,
    #[error("That language pack is larger than the 256 KiB safety limit.")]
    ManifestTooLarge,
    #[error("That file is not a valid WyrmGrid language-pack manifest.")]
    InvalidManifest,
    #[error("That language pack uses an unsupported schema or source-catalog version.")]
    UnsupportedVersion,
    #[error("That language-pack identifier is invalid or reserved by WyrmGrid.")]
    InvalidIdentifier,
    #[error("That language pack contains invalid locale, name, author, or direction metadata.")]
    InvalidMetadata,
    #[error("That language pack contains an unknown, malformed, or incompatible message.")]
    InvalidMessage,
    #[error("Community language packs cannot replace protected WyrmGrid messages.")]
    ProtectedMessage,
    #[error("Choose a language pack that is currently available.")]
    UnknownLanguagePack,
}

pub struct LanguageSettingsService<R> {
    repository: R,
}

impl<R: LanguagePackRepository> LanguageSettingsService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn status(&self) -> Result<LanguageStatus, LanguageSettingsError> {
        let source = source_catalog().clone();
        let eligible_messages = community_message_count(&source);
        let mut packs = vec![AvailableLanguagePack {
            translated_messages: eligible_messages,
            eligible_messages,
            manifest: source.clone(),
            trust: LanguagePackTrust::BuiltIn,
        }];

        for stored in self.repository.list_custom_language_packs()? {
            if let Ok(manifest) = parse_community_manifest(&stored.manifest_json)
                && manifest.id == stored.pack_id
            {
                packs.push(AvailableLanguagePack {
                    translated_messages: manifest.messages.len(),
                    eligible_messages,
                    manifest,
                    trust: LanguagePackTrust::Community,
                });
            }
        }

        let requested_id = self
            .repository
            .load_selected_language_pack()?
            .unwrap_or_else(|| DEFAULT_LANGUAGE_PACK_ID.to_owned());
        let active_pack = packs
            .iter()
            .find(|pack| pack.manifest.id == requested_id)
            .unwrap_or(&packs[0])
            .manifest
            .clone();

        Ok(LanguageStatus {
            selected_language_pack_id: active_pack.id.clone(),
            active_pack,
            packs,
        })
    }

    pub fn select(&self, pack_id: &str) -> Result<LanguageStatus, LanguageSettingsError> {
        let status = self.status()?;
        if !status.packs.iter().any(|pack| pack.manifest.id == pack_id) {
            return Err(LanguageSettingsError::UnknownLanguagePack);
        }
        self.repository.save_selected_language_pack(pack_id)?;
        self.status()
    }

    pub fn import(&self, manifest_json: &str) -> Result<LanguageStatus, LanguageSettingsError> {
        let manifest = parse_community_manifest(manifest_json)?;
        let canonical_json =
            serde_json::to_string(&manifest).map_err(|_| LanguageSettingsError::InvalidManifest)?;
        self.repository
            .save_custom_language_pack(&manifest.id, &canonical_json)?;
        self.repository.save_selected_language_pack(&manifest.id)?;
        self.status()
    }
}

pub fn source_catalog() -> &'static LanguagePackManifest {
    &SOURCE_CATALOG
}

fn parse_community_manifest(
    manifest_json: &str,
) -> Result<LanguagePackManifest, LanguageSettingsError> {
    if manifest_json.len() > MAX_LANGUAGE_PACK_BYTES {
        return Err(LanguageSettingsError::ManifestTooLarge);
    }
    let manifest: LanguagePackManifest =
        serde_json::from_str(manifest_json).map_err(|_| LanguageSettingsError::InvalidManifest)?;
    validate_manifest(&manifest, PackValidationMode::Community)?;
    Ok(manifest)
}

#[derive(Clone, Copy)]
enum PackValidationMode {
    BuiltIn,
    Community,
}

fn validate_manifest(
    manifest: &LanguagePackManifest,
    mode: PackValidationMode,
) -> Result<(), LanguageSettingsError> {
    if manifest.schema_version != LANGUAGE_PACK_SCHEMA_VERSION
        || manifest.source_catalog_version != SOURCE_CATALOG_VERSION
    {
        return Err(LanguageSettingsError::UnsupportedVersion);
    }
    if !valid_pack_id(&manifest.id)
        || (matches!(mode, PackValidationMode::Community) && manifest.id.starts_with("wyrmgrid-"))
    {
        return Err(LanguageSettingsError::InvalidIdentifier);
    }
    if !valid_locale(&manifest.locale)
        || manifest.source_locale != SOURCE_LOCALE
        || !valid_label(&manifest.name, 80)
        || manifest
            .author
            .as_ref()
            .is_some_and(|author| !valid_label(author, 80))
    {
        return Err(LanguageSettingsError::InvalidMetadata);
    }
    if manifest.messages.is_empty() || manifest.messages.len() > MAX_LANGUAGE_MESSAGES {
        return Err(LanguageSettingsError::InvalidMessage);
    }

    let source = if matches!(mode, PackValidationMode::BuiltIn) {
        manifest
    } else {
        source_catalog()
    };
    for (key, pattern) in &manifest.messages {
        let Some(source_pattern) = source.messages.get(key) else {
            return Err(LanguageSettingsError::InvalidMessage);
        };
        if matches!(mode, PackValidationMode::Community) && protected_message(key) {
            return Err(LanguageSettingsError::ProtectedMessage);
        }
        if !valid_message_key(key)
            || !valid_message_pattern(key, pattern)
            || message_variables(pattern) != message_variables(source_pattern)
        {
            return Err(LanguageSettingsError::InvalidMessage);
        }
    }
    Ok(())
}

fn community_message_count(source: &LanguagePackManifest) -> usize {
    source
        .messages
        .keys()
        .filter(|key| !protected_message(key))
        .count()
}

fn protected_message(key: &str) -> bool {
    PROTECTED_MESSAGE_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
}

fn valid_pack_id(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn valid_locale(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || value.contains('_') {
        return false;
    }
    let mut subtags = value.split('-');
    let Some(language) = subtags.next() else {
        return false;
    };
    let private_only = language.eq_ignore_ascii_case("x");
    if !private_only
        && (!(2..=8).contains(&language.len())
            || !language.bytes().all(|byte| byte.is_ascii_alphabetic()))
    {
        return false;
    }
    let remainder = subtags.collect::<Vec<_>>();
    (!private_only || !remainder.is_empty())
        && remainder.iter().all(|subtag| {
            (1..=8).contains(&subtag.len())
                && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
}

fn valid_label(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(|character| character.is_control())
}

fn valid_message_key(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn valid_message_pattern(key: &str, pattern: &str) -> bool {
    if pattern.trim().is_empty()
        || pattern.len() > MAX_MESSAGE_PATTERN_BYTES
        || pattern.contains(['<', '>'])
        || pattern.chars().any(|character| {
            (character.is_control() && !matches!(character, '\n' | '\t'))
                || matches!(
                    character,
                    '\u{202A}'
                        ..='\u{202E}' | '\u{2066}'
                        ..='\u{2069}'
                )
        })
    {
        return false;
    }
    let mut lines = pattern.lines();
    let Some(first) = lines.next() else {
        return false;
    };
    let mut resource = format!("{key} = {first}");
    for line in lines {
        resource.push_str("\n    ");
        resource.push_str(line);
    }
    parser::parse(resource.as_str()).is_ok()
}

fn message_variables(pattern: &str) -> BTreeSet<String> {
    let bytes = pattern.as_bytes();
    let mut variables = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'-' | b'_'))
        {
            end += 1;
        }
        if end > start {
            variables.insert(pattern[start..end].to_owned());
        }
        index = end.max(index + 1);
    }
    variables
}

#[cfg(test)]
#[path = "tests/localization.rs"]
mod tests;

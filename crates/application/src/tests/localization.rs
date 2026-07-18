use super::*;

const COMMUNITY_FIXTURE: &str = include_str!("../../../../schemas/fixtures/language-pack-v1.json");

#[test]
fn validates_the_built_in_and_community_catalogues() {
    assert_eq!(source_catalog().id, DEFAULT_LANGUAGE_PACK_ID);
    let manifest = parse_community_manifest(COMMUNITY_FIXTURE).unwrap();
    assert_eq!(manifest.locale, "fr");
    assert_eq!(manifest.messages.get("nav-fleet"), Some(&"Flotte".into()));
}

#[test]
fn rejects_protected_unknown_and_incompatible_messages() {
    let mut manifest: LanguagePackManifest = serde_json::from_str(COMMUNITY_FIXTURE).unwrap();
    manifest
        .messages
        .insert("error-onair-rate-limited".into(), "Réessayez".into());
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::ProtectedMessage)
    );

    manifest.messages.remove("error-onair-rate-limited");
    manifest
        .messages
        .insert("unknown-message".into(), "Inconnu".into());
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::InvalidMessage)
    );

    manifest.messages.remove("unknown-message");
    manifest.messages.insert(
        "language-coverage".into(),
        "Couverture { $translated }".into(),
    );
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::InvalidMessage)
    );
}

#[test]
fn rejects_a_pack_targeting_an_older_source_catalogue() {
    let mut manifest: LanguagePackManifest = serde_json::from_str(COMMUNITY_FIXTURE).unwrap();
    manifest.source_catalog_version = SOURCE_CATALOG_VERSION - 1;
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::UnsupportedVersion)
    );
}

#[test]
fn community_packs_cannot_relabel_security_centre_authority() {
    let mut manifest: LanguagePackManifest = serde_json::from_str(COMMUNITY_FIXTURE).unwrap();
    manifest
        .messages
        .insert("security-revoke".into(), "Conserver l’autorisation".into());
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::ProtectedMessage)
    );
}

#[test]
fn community_packs_cannot_relabel_backup_password_or_restore_controls() {
    let mut manifest: LanguagePackManifest = serde_json::from_str(COMMUNITY_FIXTURE).unwrap();
    manifest.messages.insert(
        "data-protection-restore-confirm".into(),
        "Aucun remplacement ne sera effectué".into(),
    );
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::ProtectedMessage)
    );
}

#[test]
fn imports_selects_and_restores_a_community_pack() {
    let store = Store::open_in_memory().unwrap();
    let service = LanguageSettingsService::new(store.clone());
    assert_eq!(
        service.status().unwrap().selected_language_pack_id,
        DEFAULT_LANGUAGE_PACK_ID
    );
    let imported = service.import(COMMUNITY_FIXTURE).unwrap();
    assert_eq!(imported.selected_language_pack_id, "community-fr-example");
    assert_eq!(imported.active_pack.locale, "fr");
    assert_eq!(
        LanguageSettingsService::new(store).status().unwrap(),
        imported
    );
}

#[test]
fn rejects_markup_and_dangerous_bidirectional_controls() {
    let mut manifest: LanguagePackManifest = serde_json::from_str(COMMUNITY_FIXTURE).unwrap();
    manifest
        .messages
        .insert("nav-fleet".into(), "<b>Flotte</b>".into());
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::InvalidMessage)
    );

    manifest
        .messages
        .insert("nav-fleet".into(), "Flotte\u{202E}".into());
    assert_eq!(
        validate_manifest(&manifest, PackValidationMode::Community),
        Err(LanguageSettingsError::InvalidMessage)
    );
}

#[test]
fn corrupt_stored_selection_falls_back_to_canonical_english() {
    let store = Store::open_in_memory().unwrap();
    store
        .save_custom_language_pack_record("broken-pack", "{not-json")
        .unwrap();
    store
        .save_selected_language_pack_record("broken-pack")
        .unwrap();

    let status = LanguageSettingsService::new(store).status().unwrap();
    assert_eq!(status.selected_language_pack_id, DEFAULT_LANGUAGE_PACK_ID);
    assert_eq!(status.active_pack.locale, SOURCE_LOCALE);
}

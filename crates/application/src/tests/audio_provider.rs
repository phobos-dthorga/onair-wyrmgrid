use super::*;

#[test]
fn provider_registration_rejects_unsafe_or_mismatched_manifests() {
    let invalid = r#"{
        "schema_version": 2,
        "id": "dev.wyrmgrid.fake-audio",
        "name": "Fake",
        "version": "0.2.0",
        "audio_protocol_version": 2,
        "author": "WyrmGrid",
        "entry_point": "../fake",
        "platforms": ["windows_x86_64"],
        "capabilities": ["source_enumeration"]
    }"#;
    assert!(matches!(
        AudioProviderRegistration::from_manifest_json(invalid, "fake"),
        Err(AudioProviderError::InvalidManifest)
    ));
}

#[test]
fn provider_handshake_descriptor_must_match_the_installed_manifest() {
    use wyrmgrid_audio_provider_protocol::AudioProviderCapability;

    let manifest = AudioProviderManifest {
        schema: None,
        schema_version: 2,
        id: "dev.wyrmgrid.fake-audio".into(),
        name: "Fake audio provider".into(),
        version: "0.1.0".into(),
        audio_protocol_version: 2,
        author: "WyrmGrid".into(),
        entry_point: "fake-audio-provider".into(),
        platforms: vec![current_audio_provider_platform()],
        capabilities: vec![
            AudioProviderCapability::SourceEnumeration,
            AudioProviderCapability::PcmS16leCapture,
        ],
    };
    let descriptor = AudioProviderDescriptor {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        platform: current_audio_provider_platform(),
        capabilities: manifest.capabilities.iter().rev().copied().collect(),
    };

    assert!(provider_matches_manifest(&descriptor, &manifest));

    let mut mismatched = descriptor.clone();
    mismatched.version = "0.1.1".into();
    assert!(!provider_matches_manifest(&mismatched, &manifest));

    let mut mismatched = descriptor.clone();
    mismatched.capabilities.pop();
    assert!(!provider_matches_manifest(&mismatched, &manifest));

    let mut mismatched = descriptor;
    mismatched.name = "Substituted provider".into();
    assert!(!provider_matches_manifest(&mismatched, &manifest));
}

#[test]
fn source_truth_ids_are_stable_storage_values() {
    assert_eq!(source_truth_id(AudioSourceTruth::Isolated), "isolated");
    assert_eq!(
        source_truth_id(AudioSourceTruth::MixedOutput),
        "mixed_output"
    );
    assert_eq!(
        source_truth_id(AudioSourceTruth::MetadataOnly),
        "metadata_only"
    );
}

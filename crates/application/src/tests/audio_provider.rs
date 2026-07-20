use super::*;

#[test]
fn provider_registration_rejects_unsafe_or_mismatched_manifests() {
    let invalid = r#"{
        "schema_version": 1,
        "id": "dev.wyrmgrid.fake-audio",
        "name": "Fake",
        "version": "0.2.0",
        "audio_protocol_version": 1,
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

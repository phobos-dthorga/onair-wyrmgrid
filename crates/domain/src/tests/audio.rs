use super::*;

fn microphone() -> AudioSourceCapability {
    AudioSourceCapability {
        schema_version: AUDIO_SOURCE_SCHEMA_VERSION,
        id: "synthetic.microphone.primary".into(),
        display_name: "Synthetic pilot microphone".into(),
        role: AudioSourceRole::MicrophoneInput,
        direction: AudioSourceDirection::Input,
        truth: AudioSourceTruth::Isolated,
        availability: AudioSourceAvailability::Available,
        permission: AudioPermissionState::Granted,
        channels: 1,
        native_sample_rate_hz: 48_000,
        supported_profiles: vec![AudioOpusProfileId::PilotMicrophoneV1],
        supports_hot_plug: true,
        origin: AudioSourceOrigin::OperatingSystem,
    }
}

#[test]
fn defines_the_bounded_version_one_opus_catalogue() {
    let microphone = AudioOpusProfileId::PilotMicrophoneV1.spec();
    assert_eq!(microphone.channels, 1);
    assert_eq!(microphone.sample_rate_hz, 48_000);
    assert_eq!(microphone.target_bitrate_bps, 48_000);
    assert_eq!(microphone.estimated_encoded_bytes(3_600), Some(21_600_000));

    let voice = AudioOpusProfileId::IsolatedVoiceV1.spec();
    assert_eq!(voice.channels, 1);
    assert_eq!(voice.target_bitrate_bps, 32_000);
    assert_eq!(voice.estimated_encoded_bytes(3_600), Some(14_400_000));

    let mixed = AudioOpusProfileId::MixedStereoV1.spec();
    assert_eq!(mixed.channels, 2);
    assert_eq!(mixed.target_bitrate_bps, 128_000);
    assert_eq!(mixed.estimated_encoded_bytes(3_600), Some(57_600_000));
    assert_eq!(mixed.estimated_encoded_bytes(u64::MAX), None);
}

#[test]
fn validates_a_capture_ready_source() {
    let source = microphone();
    assert_eq!(source.validate(), Ok(()));
    assert!(source.is_capture_ready());
}

#[test]
fn rejects_invalid_source_boundaries_and_duplicate_profiles() {
    let mut candidate = microphone();
    candidate.schema_version += 1;
    assert_eq!(
        candidate.validate(),
        Err(AudioSourceCapabilityError::UnsupportedSchemaVersion)
    );

    candidate = microphone();
    candidate.id = "../microphone".into();
    assert_eq!(
        candidate.validate(),
        Err(AudioSourceCapabilityError::InvalidIdentity)
    );

    candidate = microphone();
    candidate.channels = 0;
    assert_eq!(
        candidate.validate(),
        Err(AudioSourceCapabilityError::InvalidFormat)
    );

    candidate = microphone();
    candidate
        .supported_profiles
        .push(candidate.supported_profiles[0]);
    assert_eq!(
        candidate.validate(),
        Err(AudioSourceCapabilityError::InvalidProfiles)
    );

    candidate = microphone();
    candidate.supported_profiles = vec![AudioOpusProfileId::MixedStereoV1];
    assert_eq!(
        candidate.validate(),
        Err(AudioSourceCapabilityError::InvalidProfiles)
    );
}

#[test]
fn metadata_only_and_unavailable_sources_never_report_capture_ready() {
    let mut metadata = microphone();
    metadata.truth = AudioSourceTruth::MetadataOnly;
    metadata.supported_profiles.clear();
    assert_eq!(metadata.validate(), Ok(()));
    assert!(!metadata.is_capture_ready());

    let mut unavailable = microphone();
    unavailable.availability = AudioSourceAvailability::Unavailable;
    assert_eq!(unavailable.validate(), Ok(()));
    assert!(!unavailable.is_capture_ready());

    let mut denied = microphone();
    denied.permission = AudioPermissionState::Denied;
    assert_eq!(denied.validate(), Ok(()));
    assert!(!denied.is_capture_ready());
}

#[test]
fn rejects_unknown_source_fields() {
    let mut value = serde_json::to_value(microphone()).expect("source should encode");
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<AudioSourceCapability>(value).is_err());
}

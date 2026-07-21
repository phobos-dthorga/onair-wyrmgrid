use super::*;

#[test]
fn codec_registration_accepts_the_bundled_manifest_and_rejects_unsafe_paths() {
    let manifest = include_str!("../../../../codecs/opus/codec.json");
    let registration = AudioCodecRegistration::from_manifest_json(manifest, "codec.exe").unwrap();
    assert_eq!(registration.manifest.id, "dev.wyrmgrid.opus");
    assert_eq!(registration.manifest.profiles.len(), 3);

    let unsafe_manifest = manifest.replace("\"wyrmgrid-opus-codec\"", "\"../wyrmgrid-opus-codec\"");
    assert!(matches!(
        AudioCodecRegistration::from_manifest_json(&unsafe_manifest, "codec.exe"),
        Err(AudioCodecError::InvalidManifest)
    ));
}

#[test]
fn codec_executable_path_is_platform_specific_and_bounded_to_the_directory() {
    let manifest: wyrmgrid_audio_codec_protocol::AudioCodecManifest =
        serde_json::from_str(include_str!("../../../../codecs/opus/codec.json")).unwrap();
    let path = codec_executable_in(std::path::Path::new("codecs"), &manifest);
    assert_eq!(path.parent(), Some(std::path::Path::new("codecs")));
    assert!(
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("wyrmgrid-opus-codec"))
    );
}

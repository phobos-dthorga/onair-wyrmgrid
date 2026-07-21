use super::*;

#[test]
fn hashes_device_identifiers_without_exposing_the_original() {
    let source_id = stable_source_id("sensitive-operating-system-device-id");
    assert!(source_id.starts_with("windows-microphone:"));
    assert_eq!(source_id.len(), "windows-microphone:".len() + 64);
    assert!(!source_id.contains("sensitive"));
}

#[test]
fn downmixes_and_bounds_synthetic_samples_without_opening_a_device() {
    let mut samples = Vec::new();
    append_f32(&[1.5, 0.5, -1.5, -0.5], 2, &mut samples);
    assert_eq!(samples, [i16::MAX, i16::MIN + 1]);

    samples.clear();
    append_i16(&[1_000, 3_000, -1_000, -3_000], 2, &mut samples);
    assert_eq!(samples, [2_000, -2_000]);
}

#[test]
fn calculates_synthetic_levels_without_recording_audio() {
    assert_eq!(level(&[0; 16]), (-120_000, false));
    assert_eq!(level(&[i16::MAX]), (0, true));
}

use super::*;

fn context() -> AudioSegmentContext {
    AudioSegmentContext {
        session_id: "audio-session-1".into(),
        track_id: "track-1".into(),
        segment_index: 0,
        first_frame: 0,
        frame_count: 960,
    }
}

fn packets() -> Vec<EncodedAudioPacket> {
    vec![EncodedAudioPacket {
        sequence: 1,
        provider_monotonic_ns: 1_020_000,
        duration_48khz_frames: 960,
        bytes: vec![0xf8, 0xff, 0xfe, 0x00],
    }]
}

#[test]
fn encrypted_segment_round_trips_and_uses_no_plaintext_path_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let store =
        EncryptedAudioMediaStore::new(directory.path(), AudioMediaKey::from_test_bytes([7; 32]));
    let stored = store.write_segment(&context(), &packets()).unwrap();
    assert_eq!(stored.storage_key.len(), 32);
    assert!(
        stored
            .storage_key
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert_eq!(
        store
            .read_segment(&stored.storage_key, &stored.envelope_sha256, &context())
            .unwrap(),
        packets()
    );
}

#[test]
fn encrypted_segment_rejects_context_substitution_and_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let store =
        EncryptedAudioMediaStore::new(directory.path(), AudioMediaKey::from_test_bytes([9; 32]));
    let stored = store.write_segment(&context(), &packets()).unwrap();
    let mut wrong_context = context();
    wrong_context.track_id = "track-2".into();
    assert!(matches!(
        store.read_segment(&stored.storage_key, &stored.envelope_sha256, &wrong_context),
        Err(AudioMediaError::AuthenticationFailed)
    ));

    let path = directory
        .path()
        .join(&stored.storage_key[..2])
        .join(format!("{}.wga", stored.storage_key));
    let mut bytes = std::fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(path, bytes).unwrap();
    assert!(matches!(
        store.read_segment(&stored.storage_key, &stored.envelope_sha256, &context()),
        Err(AudioMediaError::AuthenticationFailed)
    ));
}

#[test]
fn encrypted_segment_rejects_the_wrong_device_key() {
    let directory = tempfile::tempdir().unwrap();
    let writer =
        EncryptedAudioMediaStore::new(directory.path(), AudioMediaKey::from_test_bytes([9; 32]));
    let reader =
        EncryptedAudioMediaStore::new(directory.path(), AudioMediaKey::from_test_bytes([10; 32]));
    let stored = writer.write_segment(&context(), &packets()).unwrap();

    assert!(matches!(
        reader.read_segment(&stored.storage_key, &stored.envelope_sha256, &context()),
        Err(AudioMediaError::AuthenticationFailed)
    ));
}

#[test]
fn recovery_removes_only_pending_and_unretained_recognized_media() {
    let directory = tempfile::tempdir().unwrap();
    let store =
        EncryptedAudioMediaStore::new(directory.path(), AudioMediaKey::from_test_bytes([11; 32]));
    let stored = store.write_segment(&context(), &packets()).unwrap();
    let retained_path = directory
        .path()
        .join(&stored.storage_key[..2])
        .join(format!("{}.wga", stored.storage_key));
    let candidate_directory = directory.path().join("aa");
    std::fs::create_dir_all(&candidate_directory).unwrap();
    let pending_path = candidate_directory.join("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.pending");
    let orphan_path = candidate_directory.join("aabbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.wga");
    let unrelated_path = candidate_directory.join("notes.pending");
    std::fs::write(&pending_path, b"pending").unwrap();
    std::fs::write(&orphan_path, b"orphan").unwrap();
    std::fs::write(&unrelated_path, b"unrelated").unwrap();
    let retained = BTreeSet::from([stored.storage_key]);

    assert_eq!(store.discard_orphan_segments(&retained).unwrap(), 2);
    assert!(retained_path.is_file());
    assert!(!pending_path.exists());
    assert!(!orphan_path.exists());
    assert!(unrelated_path.is_file());
}

#[test]
fn media_operations_reject_a_redirected_or_non_directory_root() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("audio-media-v1");
    std::fs::write(&root, b"not a directory").unwrap();
    let store = EncryptedAudioMediaStore::new(&root, AudioMediaKey::from_test_bytes([14; 32]));

    assert!(matches!(
        store.write_segment(&context(), &packets()),
        Err(AudioMediaError::UnsafeStoragePath)
    ));
    assert!(matches!(
        store.discard_orphan_segments(&BTreeSet::new()),
        Err(AudioMediaError::UnsafeStoragePath)
    ));
}

#[test]
fn packet_stream_rejects_non_monotonic_sequences() {
    let mut invalid = packets();
    invalid.push(invalid[0].clone());
    assert!(matches!(
        encode_packet_export(&invalid),
        Err(AudioMediaError::InvalidInput)
    ));
}

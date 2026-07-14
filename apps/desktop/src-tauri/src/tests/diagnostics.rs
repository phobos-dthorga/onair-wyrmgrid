use super::*;

fn temporary_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("wyrmgrid-diagnostics-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).expect("temporary diagnostic directory should be created");
    path
}

#[test]
fn persists_only_bounded_structured_english_diagnostics() {
    let directory = temporary_directory();
    let log = DiagnosticLog::open(Some(&directory));
    log.record(
        "error",
        "onair.malformed_response",
        "synchronize_onair_company_data",
        "OnAir returned malformed JSON.",
    );

    let restored = DiagnosticLog::open(Some(&directory));
    let view = restored.view();
    assert_eq!(view.language, "English");
    assert_eq!(view.storage, "local_file");
    assert_eq!(view.entries.len(), 1);
    assert_eq!(view.entries[0].code, "onair.malformed_response");
    assert!(!view.entries[0].message.contains('\n'));

    fs::remove_dir_all(directory).expect("temporary diagnostic directory should be removed");
}

#[test]
fn rotates_to_the_most_recent_entries() {
    let log = DiagnosticLog::open(None);
    for index in 0..=MAX_ENTRIES {
        log.record(
            "warning",
            "test.rotation",
            "test",
            &format!("entry {index}"),
        );
    }

    let entries = log.view().entries;
    assert_eq!(entries.len(), MAX_ENTRIES);
    assert_eq!(entries[0].message, "entry 1");
    assert_eq!(
        entries[MAX_ENTRIES - 1].message,
        format!("entry {MAX_ENTRIES}")
    );
}

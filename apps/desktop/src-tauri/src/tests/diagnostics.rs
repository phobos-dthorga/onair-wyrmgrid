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
        None,
    );

    let restored = DiagnosticLog::open(Some(&directory));
    let view = restored.view();
    assert_eq!(view.language, "English");
    assert_eq!(view.storage, "local_file");
    assert_eq!(view.entries.len(), 1);
    assert_eq!(view.entries[0].code, "onair.malformed_response");
    assert_eq!(view.entries[0].plugin_id, None);
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
            None,
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

#[test]
fn persists_only_valid_bounded_plugin_identifiers() {
    let directory = temporary_directory();
    let log = DiagnosticLog::open(Some(&directory));
    log.record(
        "error",
        "plugin.weather_product_request_mismatch",
        "plugin_weather",
        "The plugin weather product did not match the bounded host request.",
        Some("org.wyrmgrid.provider.open-meteo"),
    );
    log.record(
        "error",
        "plugin.message_unauthorized",
        "plugin_protocol",
        "The plugin sent an invalid or unauthorized message.",
        Some("plugin\noutput"),
    );

    let entries = DiagnosticLog::open(Some(&directory)).view().entries;
    assert_eq!(
        entries[0].plugin_id.as_deref(),
        Some("org.wyrmgrid.provider.open-meteo")
    );
    assert_eq!(entries[1].plugin_id, None);

    fs::remove_dir_all(directory).expect("temporary diagnostic directory should be removed");
}

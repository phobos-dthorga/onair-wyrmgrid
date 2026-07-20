use super::*;
use sentry::protocol::{Exception, Frame, User};

#[test]
fn telemetry_requires_an_explicit_true_value() {
    assert!(enabled_value("true"));
    assert!(enabled_value(" YES "));
    assert!(!enabled_value(""));
    assert!(!enabled_value("enabled"));
}

#[test]
fn metadata_is_bounded_and_low_cardinality() {
    assert!(safe_metadata("onair-wyrmgrid@0.1.0"));
    assert!(safe_metadata("maintainer-test"));
    assert!(!safe_metadata("company name"));
    assert!(!safe_metadata(&"a".repeat(81)));
}

#[test]
fn error_codes_are_bounded_and_machine_owned() {
    assert!(safe_error_code("a.1"));
    assert!(safe_error_code(&format!("a{}", "1".repeat(79))));
    assert!(!safe_error_code("a1"));
    assert!(!safe_error_code(&format!("a{}", "1".repeat(80))));
    assert!(safe_error_code("plugin.weather_product_request_mismatch"));
    assert!(!safe_error_code("Plugin failure"));
    assert!(!safe_error_code("plugin.secret=value"));
    assert!(!safe_error_code(&format!("plugin.{}", "a".repeat(80))));
}

#[test]
fn strips_sensitive_event_fields_but_keeps_a_symbolic_stack() {
    let mut event = Event {
        message: Some("api-key=secret-value".to_owned()),
        server_name: Some(Cow::Borrowed("developer-machine")),
        user: Some(User {
            id: Some("company-id".to_owned()),
            ..Default::default()
        }),
        exception: vec![Exception {
            value: Some("failed for C:\\Users\\name\\fleet.db".to_owned()),
            stacktrace: Some(Stacktrace {
                frames: vec![Frame {
                    function: Some("wyrmgrid::run".to_owned()),
                    abs_path: Some("C:\\Users\\name\\src\\main.rs".to_owned()),
                    context_line: Some("panic!(secret)".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        }]
        .into(),
        ..Default::default()
    };
    event
        .extra
        .insert("raw_payload".to_owned(), "secret".into());
    event
        .tags
        .insert("error.code".to_owned(), "plugin secret=value".to_owned());
    event.tags.insert(
        "plugin.id".to_owned(),
        "org.wyrmgrid.provider.open-meteo".to_owned(),
    );

    let sanitized = sanitize_event(event).expect("event should be retained");

    assert_eq!(sanitized.message.as_deref(), Some(SAFE_FAILURE_MESSAGE));
    assert!(sanitized.user.is_none());
    assert!(sanitized.server_name.is_none());
    assert!(sanitized.extra.is_empty());
    assert!(sanitized.tags.is_empty());
    let exception = &sanitized.exception.values[0];
    assert_eq!(exception.value.as_deref(), Some(SAFE_FAILURE_MESSAGE));
    let frame = &exception
        .stacktrace
        .as_ref()
        .expect("stacktrace should remain")
        .frames[0];
    assert_eq!(frame.function.as_deref(), Some("wyrmgrid::run"));
    assert!(frame.abs_path.is_none());
    assert!(frame.context_line.is_none());
}

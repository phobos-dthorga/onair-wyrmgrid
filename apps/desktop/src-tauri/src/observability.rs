use sentry::protocol::{Context, Event, Stacktrace};
use sentry::{ClientInitGuard, ClientOptions, Level};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

const TELEMETRY_ENABLED_ENV: &str = "WYRMGRID_SENTRY_ENABLED";
const TEST_EVENT_ENV: &str = "WYRMGRID_SENTRY_TEST_EVENT";
const RUST_DSN_ENV: &str = "SENTRY_RUST_DSN";
const SAFE_FAILURE_MESSAGE: &str = "WyrmGrid encountered an unexpected failure.";

pub struct Controller {
    guard: Mutex<Option<ClientInitGuard>>,
}

impl Controller {
    pub fn new(user_enabled: bool) -> Self {
        Self {
            guard: Mutex::new(user_enabled.then(init_client).flatten()),
        }
    }

    pub fn apply_user_preference(&self, enabled: bool) {
        let Ok(mut guard) = self.guard.lock() else {
            return;
        };
        if enabled && guard.is_none() {
            *guard = init_client();
        } else if !enabled {
            *guard = None;
        }
    }
}

fn init_client() -> Option<ClientInitGuard> {
    if !enabled_value(&std::env::var(TELEMETRY_ENABLED_ENV).unwrap_or_default()) {
        return None;
    }

    let dsn: sentry::types::Dsn = std::env::var(RUST_DSN_ENV).ok()?.parse().ok()?;
    let release = std::env::var("SENTRY_RELEASE")
        .ok()
        .filter(|value| safe_metadata(value))
        .unwrap_or_else(|| format!("onair-wyrmgrid@{}", env!("CARGO_PKG_VERSION")));
    let environment = std::env::var("SENTRY_ENVIRONMENT")
        .ok()
        .filter(|value| safe_metadata(value))
        .unwrap_or_else(|| "maintainer".to_owned());

    let guard = sentry::init((
        dsn,
        ClientOptions {
            release: Some(Cow::Owned(release)),
            environment: Some(Cow::Owned(environment)),
            send_default_pii: false,
            server_name: None,
            traces_sample_rate: 0.0,
            max_breadcrumbs: 0,
            attach_stacktrace: true,
            auto_session_tracking: false,
            enable_logs: false,
            enable_metrics: false,
            before_send: Some(Arc::new(sanitize_event)),
            before_breadcrumb: Some(Arc::new(|_| None)),
            ..Default::default()
        },
    ));
    if enabled_value(&std::env::var(TEST_EVENT_ENV).unwrap_or_default()) {
        capture_reportable("diagnostic.synthetic_test");
    }
    Some(guard)
}

pub fn capture_reportable(code: &'static str) -> Option<String> {
    sentry::Hub::current().client()?;
    let event_id = sentry::with_scope(
        |scope| scope.set_tag("error.code", code),
        || sentry::capture_message(code, Level::Error),
    );
    Some(event_id.simple().to_string())
}

fn sanitize_event(mut event: Event<'static>) -> Option<Event<'static>> {
    event.user = None;
    event.request = None;
    event.server_name = None;
    event.culprit = None;
    event.transaction = None;
    event.logger = None;
    event.logentry = None;
    event.modules.clear();
    event.extra.clear();
    event.breadcrumbs.values.clear();
    event.template = None;

    event.tags.retain(|key, _| key == "error.code");
    event.contexts.retain(|_, context| {
        matches!(
            context,
            Context::Os(_) | Context::Runtime(_) | Context::App(_)
        )
    });

    if event.message.is_some() {
        event.message = Some(
            event
                .tags
                .get("error.code")
                .cloned()
                .unwrap_or_else(|| SAFE_FAILURE_MESSAGE.to_owned()),
        );
    }

    for exception in &mut event.exception.values {
        exception.value = Some(SAFE_FAILURE_MESSAGE.to_owned());
        exception.module = None;
        if let Some(stacktrace) = &mut exception.stacktrace {
            sanitize_stacktrace(stacktrace);
        }
        if let Some(stacktrace) = &mut exception.raw_stacktrace {
            sanitize_stacktrace(stacktrace);
        }
    }
    if let Some(stacktrace) = &mut event.stacktrace {
        sanitize_stacktrace(stacktrace);
    }
    for thread in &mut event.threads.values {
        thread.name = None;
        if let Some(stacktrace) = &mut thread.stacktrace {
            sanitize_stacktrace(stacktrace);
        }
        if let Some(stacktrace) = &mut thread.raw_stacktrace {
            sanitize_stacktrace(stacktrace);
        }
    }

    Some(event)
}

fn sanitize_stacktrace(stacktrace: &mut Stacktrace) {
    stacktrace.registers.clear();
    for frame in &mut stacktrace.frames {
        frame.package = None;
        frame.filename = None;
        frame.abs_path = None;
        frame.pre_context.clear();
        frame.context_line = None;
        frame.post_context.clear();
        frame.vars.clear();
    }
}

fn enabled_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

fn safe_metadata(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'@' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
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

        let sanitized = sanitize_event(event).expect("event should be retained");

        assert_eq!(sanitized.message.as_deref(), Some(SAFE_FAILURE_MESSAGE));
        assert!(sanitized.user.is_none());
        assert!(sanitized.server_name.is_none());
        assert!(sanitized.extra.is_empty());
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
}

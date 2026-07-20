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
    if !safe_error_code(code) {
        return None;
    }
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

    event
        .tags
        .retain(|key, value| key == "error.code" && safe_error_code(value));
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

fn safe_error_code(value: &str) -> bool {
    (3..=80).contains(&value.len())
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
#[path = "tests/observability.rs"]
mod tests;

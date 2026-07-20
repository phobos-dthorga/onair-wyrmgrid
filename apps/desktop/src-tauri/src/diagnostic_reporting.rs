use crate::{diagnostics, observability};
use wyrmgrid_application::{OperationError, PluginDiagnosticEvent, PluginDiagnosticObserver};

pub struct PluginObserver;

impl PluginDiagnosticObserver for PluginObserver {
    fn observe(&self, event: &PluginDiagnosticEvent) {
        let _ = record(
            event.level,
            event.code,
            event.operation,
            event.message,
            Some(&event.plugin_id),
            event.reportable,
        );
    }
}

pub fn record(
    level: &'static str,
    code: &'static str,
    operation: &'static str,
    message: &str,
    plugin_id: Option<&str>,
    reportable: bool,
) -> Option<String> {
    diagnostics::record(level, code, operation, message, plugin_id);
    reportable
        .then(|| observability::capture_reportable(code))
        .flatten()
}

pub fn record_operation_error(
    level: &'static str,
    operation: &'static str,
    error: OperationError,
) -> OperationError {
    let report_id = record(
        level,
        error.code,
        operation,
        &error.message,
        None,
        error.reportable,
    );
    error.with_report_id(report_id)
}

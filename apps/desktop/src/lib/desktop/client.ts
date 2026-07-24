import { invoke, isTauri } from "@tauri-apps/api/core";
import { translate } from "$lib/i18n/runtime";
import type { TranslationKey } from "$lib/i18n/catalog";

export type OperationError = {
  code: string;
  message: string;
  retryable: boolean;
  reportable: boolean;
  report_id?: string;
};

export class DesktopBridgeUnavailable extends Error {
  constructor() {
    super("The WyrmGrid desktop bridge is unavailable.");
    this.name = "DesktopBridgeUnavailable";
  }
}

export class DesktopOperationFailure extends Error {
  readonly operation: OperationError;

  constructor(operation: OperationError) {
    super(operation.message);
    this.name = "DesktopOperationFailure";
    this.operation = operation;
  }
}

export function isDesktopRuntime(): boolean {
  return isTauri();
}

export async function invokeDesktop<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isDesktopRuntime()) throw new DesktopBridgeUnavailable();

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new DesktopOperationFailure(normalizeOperationError(error));
  }
}

export function operationErrorMessage(
  error: unknown,
  fallback: string,
): string {
  if (!(error instanceof DesktopOperationFailure)) return fallback;
  const messageId = operationErrorMessageKeys[error.operation.code];
  return messageId
    ? translate(messageId, {}, error.operation.message)
    : error.operation.message;
}

const operationErrorMessageKeys: Readonly<Record<string, TranslationKey>> = {
  "onair.rate_limited": "error-onair-rate-limited",
  "data_protection.reset_confirmation_required":
    "error-data-protection-reset-confirmation-required",
  "plugin.standing_permission_required":
    "error-plugin-standing-permission-required",
  "plugin.unknown_configuration": "error-plugin-unknown-configuration",
  "plugin.invalid_configuration": "error-plugin-invalid-configuration",
  "atlas.preferences_storage_unavailable":
    "error-atlas-preferences-storage-unavailable",
  "atlas.invalid_preference": "error-atlas-invalid-preference",
  "developer_resources.directory_unavailable":
    "error-developer-resources-directory-unavailable",
  "developer_resources.edk_unavailable":
    "error-developer-resources-edk-unavailable",
  "developer_resources.edk_open_failed":
    "error-developer-resources-edk-open-failed",
  "developer_resources.documentation_open_failed":
    "error-developer-resources-documentation-open-failed",
  "theme.duplicate": "error-theme-duplicate",
  "theme.bundled_delete_forbidden": "error-theme-bundled-delete",
};

function normalizeOperationError(value: unknown): OperationError {
  if (isOperationError(value)) return value;
  return {
    code: "desktop.command_failed",
    message: "WyrmGrid could not complete the desktop request.",
    retryable: true,
    reportable: false,
  };
}

function isOperationError(value: unknown): value is OperationError {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<OperationError>;
  return (
    typeof candidate.code === "string" &&
    /^[a-z][a-z0-9_.-]{2,79}$/.test(candidate.code) &&
    typeof candidate.message === "string" &&
    candidate.message.length > 0 &&
    candidate.message.length <= 500 &&
    typeof candidate.retryable === "boolean" &&
    typeof candidate.reportable === "boolean" &&
    (candidate.report_id === undefined ||
      /^[0-9a-f]{32}$/i.test(candidate.report_id))
  );
}

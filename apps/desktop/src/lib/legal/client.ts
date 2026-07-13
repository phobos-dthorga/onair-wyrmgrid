import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";

export const CURRENT_TERMS_VERSION = "2026-07-14";
export const CURRENT_PRIVACY_NOTICE_VERSION = "2026-07-14";

const PREVIEW_STORAGE_KEY = "wyrmgrid.preview.legal-preferences";

export type LegalStatus = {
  terms_version: string;
  privacy_notice_version: string;
  acknowledged: boolean;
  telemetry_enabled: boolean;
  acknowledged_at?: string;
};

export async function loadLegalStatus(): Promise<LegalStatus> {
  if (isDesktopRuntime()) return invokeDesktop<LegalStatus>("legal_status");
  return loadPreviewStatus();
}

export async function acknowledgeLegal(
  telemetryEnabled: boolean,
): Promise<LegalStatus> {
  if (isDesktopRuntime()) {
    return invokeDesktop<LegalStatus>("acknowledge_legal", {
      telemetryEnabled,
    });
  }
  return savePreviewStatus(telemetryEnabled);
}

export async function updateTelemetryPreference(
  telemetryEnabled: boolean,
): Promise<LegalStatus> {
  if (isDesktopRuntime()) {
    return invokeDesktop<LegalStatus>("update_telemetry_preference", {
      telemetryEnabled,
    });
  }
  const current = loadPreviewStatus();
  if (!current.acknowledged) {
    throw new Error("Review the current legal documents first.");
  }
  return savePreviewStatus(telemetryEnabled);
}

function loadPreviewStatus(): LegalStatus {
  const empty = currentStatus(false, false);
  try {
    const stored = JSON.parse(
      localStorage.getItem(PREVIEW_STORAGE_KEY) ?? "null",
    ) as Partial<LegalStatus> | null;
    if (!stored) return empty;
    const acknowledged =
      stored.terms_version === CURRENT_TERMS_VERSION &&
      stored.privacy_notice_version === CURRENT_PRIVACY_NOTICE_VERSION;
    return {
      ...empty,
      acknowledged,
      telemetry_enabled: acknowledged && stored.telemetry_enabled === true,
      acknowledged_at:
        acknowledged && typeof stored.acknowledged_at === "string"
          ? stored.acknowledged_at
          : undefined,
    };
  } catch {
    return empty;
  }
}

function savePreviewStatus(telemetryEnabled: boolean): LegalStatus {
  const status = currentStatus(true, telemetryEnabled);
  localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(status));
  return status;
}

function currentStatus(
  acknowledged: boolean,
  telemetryEnabled: boolean,
): LegalStatus {
  return {
    terms_version: CURRENT_TERMS_VERSION,
    privacy_notice_version: CURRENT_PRIVACY_NOTICE_VERSION,
    acknowledged,
    telemetry_enabled: acknowledged && telemetryEnabled,
    acknowledged_at: acknowledged ? new Date().toISOString() : undefined,
  };
}

import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { securityPreviewGranted } from "./sample";
import type { SecurityCentreStatus } from "./types";

export async function loadSecurityCentre(): Promise<SecurityCentreStatus> {
  return isDesktopRuntime()
    ? invokeDesktop<SecurityCentreStatus>("security_centre_status")
    : securityPreviewGranted;
}

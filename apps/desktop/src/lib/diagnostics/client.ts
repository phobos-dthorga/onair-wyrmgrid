import { invokeDesktop } from "$lib/desktop/client";
import type { DiagnosticLogView } from "./types";

export function loadDiagnosticLog(): Promise<DiagnosticLogView> {
  return invokeDesktop<DiagnosticLogView>("diagnostic_log");
}

export function clearDiagnosticLog(): Promise<DiagnosticLogView> {
  return invokeDesktop<DiagnosticLogView>("clear_diagnostic_log");
}

import { invokeDesktop } from "$lib/desktop/client";
import type {
  DispatchStatus,
  SimBriefAccountPreference,
  SimBriefReferenceKind,
} from "$lib/dispatch/types";

export function loadDispatchStatus(): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("dispatch_status");
}

export function importLatestSimBriefPlan(
  referenceKind: SimBriefReferenceKind,
  reference: string,
  rememberReference: boolean,
): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("import_simbrief_latest", {
    referenceKind,
    reference,
    rememberReference,
  });
}

export function loadSimBriefAccountPreference(): Promise<SimBriefAccountPreference | null> {
  return invokeDesktop<SimBriefAccountPreference | null>(
    "simbrief_account_preference",
  );
}

export function clearDispatchPlan(): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("clear_dispatch_plan");
}

export function refreshDispatchWeather(): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("refresh_dispatch_weather");
}

export function selectDispatchJob(jobId: string): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("select_dispatch_job", { jobId });
}

export function clearDispatchJob(): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("clear_dispatch_job");
}

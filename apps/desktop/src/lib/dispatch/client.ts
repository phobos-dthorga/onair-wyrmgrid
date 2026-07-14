import { invokeDesktop } from "$lib/desktop/client";
import type {
  DispatchStatus,
  SimBriefReferenceKind,
} from "$lib/dispatch/types";

export function loadDispatchStatus(): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("dispatch_status");
}

export function importLatestSimBriefPlan(
  referenceKind: SimBriefReferenceKind,
  reference: string,
): Promise<DispatchStatus> {
  return invokeDesktop<DispatchStatus>("import_simbrief_latest", {
    referenceKind,
    reference,
  });
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

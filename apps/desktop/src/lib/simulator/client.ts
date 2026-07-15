import { invokeDesktop } from "$lib/desktop/client";
import type {
  SimulatorBridgeView,
  SimulatorPreferences,
  SimulatorRecordingPreferences,
  SimulatorRecordingView,
  SimulatorRecordingExport,
  SimulatorSessionView,
  SimulatorSessionDebrief,
} from "./types";

export function loadSimulatorBridge(): Promise<SimulatorBridgeView> {
  return invokeDesktop("simulator_bridge_status");
}

export function startSimulatorProvider(
  providerId: string,
): Promise<SimulatorBridgeView> {
  return invokeDesktop("start_simulator_provider", { providerId });
}

export function stopSimulatorProvider(
  providerId: string,
): Promise<SimulatorBridgeView> {
  return invokeDesktop("stop_simulator_provider", { providerId });
}

export function loadSimulatorPreferences(): Promise<SimulatorPreferences> {
  return invokeDesktop("simulator_preferences");
}

export function saveSimulatorPreferences(
  preferences: SimulatorPreferences,
): Promise<SimulatorPreferences> {
  return invokeDesktop("update_simulator_preferences", { preferences });
}

export function loadSimulatorRecording(): Promise<SimulatorRecordingView> {
  return invokeDesktop("simulator_recording_status");
}

export function saveSimulatorRecordingPreferences(
  preferences: SimulatorRecordingPreferences,
): Promise<SimulatorRecordingPreferences> {
  return invokeDesktop("update_simulator_recording_preferences", {
    preferences,
  });
}

export function startSimulatorRecording(): Promise<SimulatorRecordingView> {
  return invokeDesktop("start_simulator_recording");
}

export function stopSimulatorRecording(): Promise<SimulatorRecordingView> {
  return invokeDesktop("stop_simulator_recording");
}

export function loadSimulatorRecordingSession(
  sessionId: string,
  sampleOffset = 0,
): Promise<SimulatorSessionView> {
  return invokeDesktop("simulator_recording_session", {
    sessionId,
    sampleOffset,
  });
}

export function loadSimulatorRecordingDebrief(
  sessionId: string,
): Promise<SimulatorSessionDebrief> {
  return invokeDesktop("simulator_recording_debrief", { sessionId });
}

export function pinSimulatorRecording(
  sessionId: string,
  pinned: boolean,
): Promise<SimulatorRecordingView> {
  return invokeDesktop("pin_simulator_recording", { sessionId, pinned });
}

export function exportSimulatorRecording(
  sessionId: string,
  format: "json" | "csv",
): Promise<SimulatorRecordingExport> {
  return invokeDesktop("export_simulator_recording", { sessionId, format });
}

export function deleteSimulatorRecording(
  sessionId: string,
): Promise<SimulatorRecordingView> {
  return invokeDesktop("delete_simulator_recording", { sessionId });
}

export function deleteAllSimulatorRecordings(): Promise<SimulatorRecordingView> {
  return invokeDesktop("delete_all_simulator_recordings");
}

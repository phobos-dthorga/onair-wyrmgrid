import { invokeDesktop } from "$lib/desktop/client";
import type { SimulatorBridgeView, SimulatorPreferences } from "./types";

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

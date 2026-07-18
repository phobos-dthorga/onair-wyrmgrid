import type { TranslationKey } from "$lib/i18n/catalog";
import type { ProviderConnectionState } from "./types";

export const providerConnectionStateMessageKeys = {
  starting: "simulator-state-starting",
  waiting_for_simulator: "simulator-state-waiting-for-simulator",
  connected: "simulator-state-connected",
  disconnected: "simulator-state-disconnected",
  stopped: "simulator-state-stopped",
  failed: "simulator-state-failed",
  unavailable: "simulator-state-unavailable",
} as const satisfies Record<ProviderConnectionState, TranslationKey>;

export const providerDetailMessageKeys: Readonly<
  Record<string, TranslationKey>
> = {
  "provider.executable_unavailable":
    "error-simulator-provider-executable-unavailable",
  "provider.handshake_failed": "error-simulator-provider-handshake",
  "provider.protocol_violation": "error-simulator-provider-protocol",
  "provider.stream_closed": "error-simulator-provider-connection",
  "provider.write_failed": "error-simulator-provider-connection",
  "provider.starting": "simulator-detail-starting",
  "provider.stopped": "simulator-detail-stopped",
  "provider.unsupported_platform": "simulator-detail-unsupported-platform",
  "simconnect.client_unavailable": "error-simconnect-client-unavailable",
  "simconnect.client_load_failed": "error-simconnect-client-unavailable",
  "simconnect.waiting_for_simulator": "simulator-detail-waiting",
  "simconnect.connected": "simulator-detail-connected",
  "simconnect.disconnected": "simulator-detail-disconnected",
  "simconnect.setup_failed": "error-simulator-provider-protocol",
  "simconnect.protocol_error": "error-simulator-provider-protocol",
};

export const providerDetailFallbackMessageKey =
  "simulator-detail-status-update" satisfies TranslationKey;

export const simulatorRitualMessageKeys = [
  "simulator-ritual-wake",
  "simulator-ritual-identity",
  "telemetry-simulator-ritual-capability",
  "simulator-ritual-link",
] as const satisfies readonly TranslationKey[];

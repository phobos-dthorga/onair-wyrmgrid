import { describe, expect, it } from "vitest";
import {
  providerConnectionStateMessageKeys,
  simulatorRitualMessageKeys,
} from "./bridgePresentation";

describe("simulator bridge presentation keys", () => {
  it("maps every provider connection state explicitly", () => {
    expect(Object.keys(providerConnectionStateMessageKeys)).toEqual([
      "starting",
      "waiting_for_simulator",
      "connected",
      "disconnected",
      "stopped",
      "failed",
      "unavailable",
    ]);
  });

  it("keeps the connection ritual ordered and catalogue-backed", () => {
    expect(simulatorRitualMessageKeys).toHaveLength(4);
  });
});

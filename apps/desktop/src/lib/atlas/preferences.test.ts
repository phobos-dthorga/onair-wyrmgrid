import { beforeEach, describe, expect, it, vi } from "vitest";

const desktopBridge = vi.hoisted(() => ({
  active: false,
  failUpdate: false,
  calls: [] as Array<{ command: string; args?: Record<string, unknown> }>,
  preferences: undefined as unknown,
}));

vi.mock("$lib/desktop/client", () => ({
  isDesktopRuntime: () => desktopBridge.active,
  invokeDesktop: async (
    command: string,
    args?: Record<string, unknown>,
  ): Promise<unknown> => {
    desktopBridge.calls.push({ command, args });
    if (command === "atlas_preferences") return desktopBridge.preferences;
    if (command === "update_atlas_preferences") {
      if (desktopBridge.failUpdate) throw new Error("save failed");
      desktopBridge.preferences = args?.preferences;
      return desktopBridge.preferences;
    }
    throw new Error(`Unexpected desktop command: ${command}`);
  },
}));
import {
  defaultAtlasPreferences,
  automaticSyncDelayMs,
  validateAtlasPreferences,
  validateAtlasView,
  loadAtlasPreferences,
} from "./preferences";

beforeEach(() => {
  const values = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
      clear: () => values.clear(),
      key: (index: number) => [...values.keys()][index] ?? null,
      get length() {
        return values.size;
      },
    } satisfies Storage,
  });
  desktopBridge.active = false;
  desktopBridge.failUpdate = false;
  desktopBridge.calls = [];
  desktopBridge.preferences = structuredClone(defaultAtlasPreferences);
});

describe("Atlas preferences", () => {
  it("preserves the current defaults without restoring a camera", () => {
    expect(validateAtlasPreferences(null)).toEqual(defaultAtlasPreferences);
    expect(defaultAtlasPreferences.automatic_sync_minutes).toBe(30);
    expect(Object.values(defaultAtlasPreferences.layers).every(Boolean)).toBe(
      true,
    );
    expect(defaultAtlasPreferences.restore_last_view).toBe(false);
  });

  it("rejects unsupported synchronization intervals and partial layers", () => {
    expect(
      validateAtlasPreferences({
        ...defaultAtlasPreferences,
        automatic_sync_minutes: 7,
      }),
    ).toEqual(defaultAtlasPreferences);
    expect(
      validateAtlasPreferences({
        ...defaultAtlasPreferences,
        layers: { fleet: false },
      }),
    ).toEqual(defaultAtlasPreferences);
    expect(automaticSyncDelayMs(0)).toBeUndefined();
    expect(automaticSyncDelayMs(15)).toBe(900_000);
    expect(automaticSyncDelayMs(7)).toBeUndefined();
  });

  it("normalizes wrapped camera angles and rejects unsafe values", () => {
    expect(
      validateAtlasView({
        longitude: 541,
        latitude: -33,
        zoom: 6,
        bearing: 370,
        pitch: 35,
      }),
    ).toEqual({
      longitude: -179,
      latitude: -33,
      zoom: 6,
      bearing: 10,
      pitch: 35,
    });
    expect(() =>
      validateAtlasView({
        longitude: 0,
        latitude: 91,
        zoom: 6,
        bearing: 0,
        pitch: 0,
      }),
    ).toThrow();
  });

  it("copies the old browser sync choice once and removes it after saving", async () => {
    desktopBridge.active = true;
    localStorage.setItem("wyrmgrid.atlas.automatic-sync-minutes", "60");

    await expect(loadAtlasPreferences()).resolves.toMatchObject({
      automatic_sync_minutes: 60,
    });
    expect(desktopBridge.calls.map(({ command }) => command)).toEqual([
      "atlas_preferences",
      "update_atlas_preferences",
    ]);
    expect(
      localStorage.getItem("wyrmgrid.atlas.automatic-sync-minutes"),
    ).toBeNull();
  });

  it("keeps the old browser choice when the encrypted save fails", async () => {
    desktopBridge.active = true;
    desktopBridge.failUpdate = true;
    localStorage.setItem("wyrmgrid.atlas.automatic-sync-minutes", "120");

    await expect(loadAtlasPreferences()).rejects.toThrow("save failed");
    expect(localStorage.getItem("wyrmgrid.atlas.automatic-sync-minutes")).toBe(
      "120",
    );
  });
});

import assert from "node:assert/strict";
import test from "node:test";
import {
  invokedDesktopCommands,
  registeredTauriCommands,
} from "./audit-desktop-commands.mjs";

test("extracts the registered Tauri command surface", () => {
  assert.deepEqual(
    [
      ...registeredTauriCommands(
        "tauri::generate_handler![status, save_item])",
      ),
    ],
    ["status", "save_item"],
  );
});

test("extracts literal desktop invocations with and without result types", () => {
  assert.deepEqual(
    invokedDesktopCommands(
      'invokeDesktop("status"); invokeDesktop<Result>("save_item", { id });',
    ),
    ["status", "save_item"],
  );
});

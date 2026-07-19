import { open, save } from "@tauri-apps/plugin-dialog";
import { invokeDesktop } from "$lib/desktop/client";
import type {
  DataProtectionStatus,
  LocalDataResetView,
  PortableBackupView,
  PortableRestoreView,
} from "./types";

const backupFilter = {
  name: "WyrmGrid portable backup",
  extensions: ["wyrmbackup"],
};

export function loadDataProtectionStatus(): Promise<DataProtectionStatus> {
  return invokeDesktop("data_protection_status");
}

export function resetLocalData(
  confirmation: string,
): Promise<LocalDataResetView> {
  return invokeDesktop<LocalDataResetView>("reset_local_data", {
    confirmation,
  });
}

export async function choosePortableBackupDestination(): Promise<
  string | null
> {
  return save({
    defaultPath: suggestedBackupName(),
    filters: [backupFilter],
  });
}

export async function choosePortableBackupSource(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [backupFilter],
  });
  return typeof selected === "string" ? selected : null;
}

export function createPortableBackup(
  destination: string,
  password: string,
  passwordConfirmation: string,
): Promise<PortableBackupView> {
  return invokeDesktop("create_portable_backup", {
    destination,
    password,
    passwordConfirmation,
  });
}

export function preparePortableRestore(
  source: string,
  password: string,
  replacementConfirmed: boolean,
): Promise<PortableRestoreView> {
  return invokeDesktop("prepare_portable_restore", {
    source,
    password,
    replacementConfirmed,
  });
}

function suggestedBackupName(): string {
  const date = new Date().toISOString().slice(0, 10);
  return `WyrmGrid-${date}.wyrmbackup`;
}

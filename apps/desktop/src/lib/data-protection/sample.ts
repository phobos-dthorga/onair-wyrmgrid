import type { DataProtectionStatus } from "./types";

export const browserDataProtectionStatus: DataProtectionStatus = {
  database_encrypted: true,
  device_key_protected: true,
  portable_backup_format_version: 1,
  pending_restore: false,
  local_data_reset_confirmation: "ERASE WYRMGRID DATA",
};

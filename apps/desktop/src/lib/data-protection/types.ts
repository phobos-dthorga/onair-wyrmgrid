export type DataProtectionStatus = {
  database_encrypted: boolean;
  device_key_protected: boolean;
  portable_backup_format_version: number;
  pending_restore: boolean;
  local_data_reset_confirmation: string;
};

export type PortableBackupView = {
  format_version: number;
  schema_version: number;
  created_at: string;
  application_version: string;
};

export type PortableRestoreView = {
  format_version: number;
  schema_version: number;
  backup_created_at: string;
  backup_application_version: string;
  restart_required: boolean;
};

export type LocalDataResetView = {
  restart_required: boolean;
};

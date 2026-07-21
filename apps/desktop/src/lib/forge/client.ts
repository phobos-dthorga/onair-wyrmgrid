import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { forgePreviewStopped } from "./sample";
import type {
  AuthorizationGrantLifetime,
  ManagedPluginPackage,
  PluginHostView,
  PluginPackageInspection,
} from "./types";

export async function loadPluginHost(): Promise<PluginHostView> {
  return isDesktopRuntime()
    ? invokeDesktop<PluginHostView>("plugin_host_status")
    : forgePreviewStopped;
}

export function approvePluginPermissions(
  pluginId: string,
  lifetime: AuthorizationGrantLifetime,
): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("approve_plugin_permissions", {
    pluginId,
    lifetime,
  });
}

export function revokePluginPermissions(
  pluginId: string,
): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("revoke_plugin_permissions", {
    pluginId,
  });
}

export function updatePluginStartupPreference(
  pluginId: string,
  enabled: boolean,
): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("update_plugin_startup_preference", {
    pluginId,
    enabled,
  });
}

export function updatePluginConfiguration(
  pluginId: string,
  settingKey: string,
  value: string,
): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("update_plugin_configuration", {
    pluginId,
    settingKey,
    value,
  });
}

export function startPlugin(pluginId: string): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("start_plugin", { pluginId });
}

export function stopPlugin(pluginId: string): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("stop_plugin", { pluginId });
}

export async function choosePluginPackage(): Promise<string | null> {
  if (!isDesktopRuntime()) return null;
  const selected = await openFile({
    multiple: false,
    directory: false,
    filters: [{ name: "WyrmGrid plugin package", extensions: ["wyrmplugin"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export function inspectPluginPackageFile(
  source: string,
): Promise<PluginPackageInspection> {
  return invokeDesktop<PluginPackageInspection>("inspect_plugin_package_file", {
    source,
  });
}

export function loadManagedPluginPackages(): Promise<ManagedPluginPackage[]> {
  return invokeDesktop<ManagedPluginPackage[]>("managed_plugin_packages");
}

export function installPluginPackageFile(
  source: string,
): Promise<ManagedPluginPackage> {
  return invokeDesktop<ManagedPluginPackage>("install_plugin_package_file", {
    source,
  });
}

export function setManagedPluginEnabled(
  pluginId: string,
  enabled: boolean,
): Promise<ManagedPluginPackage> {
  return invokeDesktop<ManagedPluginPackage>("set_managed_plugin_enabled", {
    pluginId,
    enabled,
  });
}

export function rollbackManagedPlugin(
  pluginId: string,
): Promise<ManagedPluginPackage> {
  return invokeDesktop<ManagedPluginPackage>("rollback_managed_plugin", {
    pluginId,
  });
}

export function removeManagedPlugin(pluginId: string): Promise<void> {
  return invokeDesktop<void>("remove_managed_plugin", { pluginId });
}

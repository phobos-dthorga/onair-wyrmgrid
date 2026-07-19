import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { forgePreviewStopped } from "./sample";
import type { AuthorizationGrantLifetime, PluginHostView } from "./types";

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

export function startPlugin(pluginId: string): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("start_plugin", { pluginId });
}

export function stopPlugin(pluginId: string): Promise<PluginHostView> {
  return invokeDesktop<PluginHostView>("stop_plugin", { pluginId });
}

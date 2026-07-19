import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";

export type StartupOptions = {
  no_launch_art: boolean;
  compact_ui: boolean;
  low_resource: boolean;
  weather_gallery: boolean;
};

export const defaultStartupOptions: StartupOptions = {
  no_launch_art: false,
  compact_ui: false,
  low_resource: false,
  weather_gallery: false,
};

export async function loadStartupOptions(): Promise<StartupOptions> {
  if (!isDesktopRuntime()) return defaultStartupOptions;
  return invokeDesktop<StartupOptions>("startup_options");
}

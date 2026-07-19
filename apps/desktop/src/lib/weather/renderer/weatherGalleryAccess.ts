import type { StartupOptions } from "$lib/launch/startup";

export function weatherGalleryAccessEnabled(
  developmentBuild: boolean,
  desktopRuntime: boolean,
  startupOptions: StartupOptions | undefined,
): boolean {
  return (
    developmentBuild ||
    (desktopRuntime && startupOptions?.weather_gallery === true)
  );
}

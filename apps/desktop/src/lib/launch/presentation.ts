export const MINIMUM_LAUNCH_DISPLAY_MS = 800;

export type LaunchArtworkTone = "dark" | "light";
export type ViewportPresentation = "standard" | "narrow" | "short";

export function viewportPresentation(
  width: number,
  height: number,
): ViewportPresentation {
  if (!Number.isFinite(width) || !Number.isFinite(height)) return "standard";
  if (width <= 900) return "narrow";
  if (height <= 720) return "short";
  return "standard";
}

export function shouldRenderLaunchArtwork(
  startupOptionsLoaded: boolean,
  noLaunchArt: boolean,
): boolean {
  return startupOptionsLoaded && !noLaunchArt;
}

export function launchArtworkTone(canvas: string): LaunchArtworkTone {
  const match = /^#([0-9a-f]{6})$/i.exec(canvas.trim());
  if (!match) return "dark";

  const value = match[1];
  const channels = [0, 2, 4].map((offset) => {
    const channel = Number.parseInt(value.slice(offset, offset + 2), 16) / 255;
    return channel <= 0.04045
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4;
  });
  const luminance =
    0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
  return luminance >= 0.45 ? "light" : "dark";
}

export function remainingLaunchDisplayTime(
  startedAt: number,
  now: number,
  minimumDisplayMs = MINIMUM_LAUNCH_DISPLAY_MS,
): number {
  if (![startedAt, now, minimumDisplayMs].every(Number.isFinite)) return 0;
  return Math.max(0, minimumDisplayMs - Math.max(0, now - startedAt));
}

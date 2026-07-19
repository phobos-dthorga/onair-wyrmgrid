import { describe, expect, it } from "vitest";
import { defaultStartupOptions } from "$lib/launch/startup";
import { weatherGalleryAccessEnabled } from "./weatherGalleryAccess";

describe("weather gallery access", () => {
  it("is available automatically in development builds", () => {
    expect(weatherGalleryAccessEnabled(true, false, undefined)).toBe(true);
  });

  it("requires the explicit desktop startup flag in release builds", () => {
    expect(
      weatherGalleryAccessEnabled(false, true, {
        ...defaultStartupOptions,
        weather_gallery: true,
      }),
    ).toBe(true);
    expect(
      weatherGalleryAccessEnabled(false, true, defaultStartupOptions),
    ).toBe(false);
  });

  it("does not trust a browser query or missing desktop bridge", () => {
    expect(
      weatherGalleryAccessEnabled(false, false, {
        ...defaultStartupOptions,
        weather_gallery: true,
      }),
    ).toBe(false);
  });
});

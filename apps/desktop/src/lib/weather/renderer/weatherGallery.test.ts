import { describe, expect, it } from "vitest";
import {
  buildWeatherGalleryUpdate,
  projectWeatherGalleryPoint,
} from "./weatherGallery";

describe("developer weather gallery", () => {
  it("provides one stable reference cell for every supported effect", () => {
    const update = buildWeatherGalleryUpdate("cinematic", "all", false);

    expect(update.scene.cells.map((cell) => cell.effect)).toEqual([
      "cloud",
      "rain",
      "convective",
      "snow",
      "obscuration",
      "dust",
    ]);
    expect(update.policy.animation).toBe(false);
    expect(update.policy.lightningFlashes).toBe(false);
  });

  it("filters without changing the reference identity", () => {
    const rain = buildWeatherGalleryUpdate("enhanced", "rain", true);

    expect(rain.scene.cells).toHaveLength(1);
    expect(rain.scene.cells[0].id).toBe("gallery-rain");
    expect(rain.scene.cells[0].longitude).toBe(0);
    expect(rain.scene.cells[0].latitude).toBe(0);
    expect(rain.policy.profile).toBe("enhanced");
  });

  it("projects reference coordinates into the isolated canvas", () => {
    expect(projectWeatherGalleryPoint(0, 0, 1_000, 600)).toEqual({
      x: 500,
      y: 300,
      surfaceVisibility: 1,
    });
    const upperLeft = projectWeatherGalleryPoint(-100, 30, 1_000, 600);
    expect(upperLeft.x).toBeLessThan(500);
    expect(upperLeft.y).toBeLessThan(300);
  });
});

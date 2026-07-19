import { describe, expect, it, vi } from "vitest";
import { weatherVisualSurfaceVisibility } from "./surfaceClipping";

describe("weather visual surface clipping", () => {
  it("samples the full visual perimeter", () => {
    const visibilityAt = vi.fn(() => 1);

    expect(weatherVisualSurfaceVisibility(1, 300, 200, 80, visibilityAt)).toBe(
      1,
    );
    expect(visibilityAt).toHaveBeenCalledTimes(8);
  });

  it("rejects a visual when any perimeter sample leaves the globe", () => {
    expect(
      weatherVisualSurfaceVisibility(1, 300, 200, 80, (x) => (x > 350 ? 0 : 1)),
    ).toBe(0);
  });

  it("preserves the lowest centre or perimeter fade", () => {
    expect(
      weatherVisualSurfaceVisibility(0.7, 300, 200, 80, () => 0.8),
    ).toBeCloseTo(0.7);
    expect(
      weatherVisualSurfaceVisibility(1, 300, 200, 80, () => 0.45),
    ).toBeCloseTo(0.45);
  });
});

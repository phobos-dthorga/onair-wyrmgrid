import { describe, expect, it } from "vitest";
import {
  weatherProjectionSurfaceVisibility,
  weatherScreenSurfaceVisibility,
} from "./projectionVisibility";

describe("weather projection surface visibility", () => {
  it("keeps exact and antimeridian-equivalent round trips visible", () => {
    expect(
      weatherProjectionSurfaceVisibility(
        { longitude: 151.2093, latitude: -33.8688 },
        { longitude: 151.2093, latitude: -33.8688 },
      ),
    ).toBe(1);
    expect(
      weatherProjectionSurfaceVisibility(
        { longitude: 179.9, latitude: 12 },
        { longitude: -180.1, latitude: 12 },
      ),
    ).toBe(1);
  });

  it("smoothly fades a projection mismatch near the horizon", () => {
    const near = weatherProjectionSurfaceVisibility(
      { longitude: 10, latitude: 20 },
      { longitude: 10.35, latitude: 20 },
    );
    const farther = weatherProjectionSurfaceVisibility(
      { longitude: 10, latitude: 20 },
      { longitude: 10.9, latitude: 20 },
    );

    expect(near).toBeGreaterThan(farther);
    expect(near).toBeGreaterThan(0);
    expect(farther).toBeLessThan(1);
  });

  it("hides large mismatches and rejects invalid coordinates", () => {
    expect(
      weatherProjectionSurfaceVisibility(
        { longitude: 0, latitude: 0 },
        { longitude: 8, latitude: 0 },
      ),
    ).toBe(0);
    expect(
      weatherProjectionSurfaceVisibility(
        { longitude: Number.NaN, latitude: 0 },
        { longitude: 0, latitude: 0 },
      ),
    ).toBe(0);
    expect(
      weatherProjectionSurfaceVisibility(
        { longitude: 0, latitude: 91 },
        { longitude: 0, latitude: 90 },
      ),
    ).toBe(0);
  });
});

describe("weather screen surface visibility", () => {
  it("keeps round-trippable pixels fully visible", () => {
    expect(
      weatherScreenSurfaceVisibility(
        { x: 400, y: 240 },
        { x: 400.25, y: 240.25 },
      ),
    ).toBe(1);
  });

  it("fades and then rejects pixels outside the projected surface", () => {
    expect(
      weatherScreenSurfaceVisibility({ x: 400, y: 240 }, { x: 405, y: 240 }),
    ).toBeGreaterThan(0);
    expect(
      weatherScreenSurfaceVisibility({ x: 400, y: 240 }, { x: 405, y: 240 }),
    ).toBeLessThan(1);
    expect(
      weatherScreenSurfaceVisibility({ x: 400, y: 240 }, { x: 420, y: 240 }),
    ).toBe(0);
  });

  it("fails closed for invalid projection results", () => {
    expect(
      weatherScreenSurfaceVisibility(
        { x: 400, y: 240 },
        { x: Number.NaN, y: 240 },
      ),
    ).toBe(0);
  });
});

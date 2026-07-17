import { describe, expect, it } from "vitest";
import { surfaceReaction } from "./responsiveSurface";

describe("responsive surface motion", () => {
  const bounds = { left: 100, top: 200, width: 400, height: 200 };

  it("rests at the centre without shifting the surface", () => {
    expect(surfaceReaction(300, 300, bounds)).toEqual({
      shiftX: 0,
      shiftY: 0,
      glowX: 50,
      glowY: 50,
    });
  });

  it("keeps movement bounded when the pointer leaves the surface", () => {
    expect(surfaceReaction(900, -100, bounds)).toEqual({
      shiftX: 1.4,
      shiftY: -1.4,
      glowX: 85,
      glowY: 15,
    });
  });

  it("does not move a surface without measurable bounds", () => {
    expect(surfaceReaction(300, 300, { ...bounds, width: 0 })).toEqual({
      shiftX: 0,
      shiftY: 0,
      glowX: 50,
      glowY: 50,
    });
  });
});

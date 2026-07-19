import { describe, expect, it } from "vitest";
import {
  PRECIPITATION_FIELD_HEIGHT,
  PRECIPITATION_FIELD_TOP,
  PRECIPITATION_FIELD_WIDTH,
  precipitationParticleTaper,
  precipitationVerticalPosition,
} from "./precipitationLayout";

describe("weather precipitation layout", () => {
  it("keeps falling particles below the cloud body", () => {
    for (const seconds of [0, 0.5, 10, 100]) {
      const y = precipitationVerticalPosition(17, seconds, 48);
      expect(y).toBeGreaterThanOrEqual(PRECIPITATION_FIELD_TOP);
      expect(y).toBeLessThan(
        PRECIPITATION_FIELD_TOP + PRECIPITATION_FIELD_HEIGHT,
      );
    }
  });

  it("tapers the precipitation curtain at its sides and vertical ends", () => {
    const middleY = PRECIPITATION_FIELD_TOP + PRECIPITATION_FIELD_HEIGHT / 2;
    expect(precipitationParticleTaper(0, middleY)).toBeCloseTo(1);
    expect(
      precipitationParticleTaper(PRECIPITATION_FIELD_WIDTH / 2, middleY),
    ).toBe(0);
    expect(precipitationParticleTaper(0, PRECIPITATION_FIELD_TOP)).toBe(0);
    expect(
      precipitationParticleTaper(
        0,
        PRECIPITATION_FIELD_TOP + PRECIPITATION_FIELD_HEIGHT,
      ),
    ).toBe(0);
  });
});

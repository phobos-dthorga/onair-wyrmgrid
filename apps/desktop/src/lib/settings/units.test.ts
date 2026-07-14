import { describe, expect, it } from "vitest";
import {
  presentAltitude,
  presentFuel,
  presentSpeed,
  presentWeight,
} from "./units";

describe("measurement presentation", () => {
  it("converts each category independently", () => {
    expect(presentAltitude(1_000, "metres").value).toBeCloseTo(304.8);
    expect(presentSpeed(100, "kilometres_per_hour").value).toBeCloseTo(185.2);
    expect(presentWeight(1_000, "kilograms").value).toBeCloseTo(453.59237);
    expect(presentFuel(undefined, 10, "litres").value).toBeCloseTo(37.85411784);
  });

  it("does not infer fuel volume from weight or weight from volume", () => {
    expect(presentFuel(100, undefined, "litres").value).toBeUndefined();
    expect(presentFuel(undefined, 20, "kilograms").value).toBeUndefined();
  });

  it("withholds missing and non-finite source values", () => {
    expect(presentAltitude(undefined, "feet").value).toBeUndefined();
    expect(presentSpeed(Number.NaN, "knots").value).toBeUndefined();
    expect(
      presentWeight(Number.POSITIVE_INFINITY, "pounds").value,
    ).toBeUndefined();
  });
});

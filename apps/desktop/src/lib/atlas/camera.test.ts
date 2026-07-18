import { describe, expect, it } from "vitest";
import { ATLAS_HOME_CENTER, balancedOverviewCoordinates } from "./camera";

describe("Atlas camera framing", () => {
  it("starts with the globe centred on the equator", () => {
    expect(ATLAS_HOME_CENTER).toEqual([0, 0]);
  });

  it("balances a global northern-hemisphere overview around the equator", () => {
    expect(
      balancedOverviewCoordinates([
        [-150, 22],
        [80, 70],
      ]),
    ).toEqual([
      [-150, 22],
      [80, 70],
      [-35, -70],
      [-35, 70],
    ]);
  });

  it("preserves a local camera fit", () => {
    const coordinates: [number, number][] = [
      [4.75, 52.3],
      [5.8, 53.1],
    ];
    expect(balancedOverviewCoordinates(coordinates)).toEqual(coordinates);
  });
});

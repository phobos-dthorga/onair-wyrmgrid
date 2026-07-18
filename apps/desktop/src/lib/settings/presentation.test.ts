import { describe, expect, it } from "vitest";
import { displayPresets } from "./types";
import {
  displayPresetMessageKeys,
  weatherProfileDetailMessageKeys,
} from "./presentation";

describe("settings presentation keys", () => {
  it("defines an explicit catalogue key for every unit preset", () => {
    expect(Object.keys(displayPresetMessageKeys)).toEqual(
      Object.keys(displayPresets),
    );
  });

  it("defines every supported weather profile without constructing keys", () => {
    expect(Object.keys(weatherProfileDetailMessageKeys)).toEqual([
      "compatibility",
      "enhanced",
      "cinematic",
    ]);
  });
});

import { describe, expect, it } from "vitest";
import { AdaptiveWeatherQualityController } from "./adaptiveQuality";

function recordMany(
  controller: AdaptiveWeatherQualityController,
  durationMs: number,
  count: number,
): void {
  for (let index = 0; index < count; index += 1) {
    controller.recordSubmission(durationMs);
  }
}

describe("adaptive weather quality", () => {
  it("degrades only after sustained renderer submission pressure", () => {
    const controller = new AdaptiveWeatherQualityController();

    recordMany(controller, 14, 11);
    expect(controller.quality).toBe("full");
    controller.recordSubmission(14);
    expect(controller.quality).toBe("balanced");
    recordMany(controller, 14, 12);
    expect(controller.quality).toBe("minimum");
  });

  it("recovers gradually after a long healthy period", () => {
    const controller = new AdaptiveWeatherQualityController();
    recordMany(controller, 14, 24);
    expect(controller.quality).toBe("minimum");

    recordMany(controller, 2, 239);
    expect(controller.quality).toBe("minimum");
    controller.recordSubmission(2);
    expect(controller.quality).toBe("balanced");
    recordMany(controller, 2, 240);
    expect(controller.quality).toBe("full");
  });

  it("ignores invalid and suspension-sized samples", () => {
    const controller = new AdaptiveWeatherQualityController();
    recordMany(controller, Number.NaN, 20);
    recordMany(controller, 1_000, 20);
    expect(controller.quality).toBe("full");
  });
});

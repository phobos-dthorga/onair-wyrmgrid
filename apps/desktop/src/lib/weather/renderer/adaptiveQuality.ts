import type { AdaptiveWeatherQuality } from "./quality";

const OVERLOAD_THRESHOLD_MS = 10;
const HEALTHY_THRESHOLD_MS = 4;
const OVERLOAD_SAMPLE_LIMIT = 12;
const HEALTHY_SAMPLE_LIMIT = 240;
const SAMPLE_SMOOTHING = 0.12;
const IGNORED_SAMPLE_CEILING_MS = 250;

function lowerQuality(quality: AdaptiveWeatherQuality): AdaptiveWeatherQuality {
  if (quality === "full") return "balanced";
  return "minimum";
}

function raiseQuality(quality: AdaptiveWeatherQuality): AdaptiveWeatherQuality {
  if (quality === "minimum") return "balanced";
  return "full";
}

/**
 * Uses renderer submission cost as a conservative pressure signal. It does not
 * claim to measure GPU execution time and deliberately changes quality slowly.
 */
export class AdaptiveWeatherQualityController {
  private qualityValue: AdaptiveWeatherQuality = "full";
  private averageSubmissionMs: number | undefined;
  private overloadSamples = 0;
  private healthySamples = 0;

  get quality(): AdaptiveWeatherQuality {
    return this.qualityValue;
  }

  recordSubmission(durationMs: number): boolean {
    if (
      !Number.isFinite(durationMs) ||
      durationMs < 0 ||
      durationMs > IGNORED_SAMPLE_CEILING_MS
    ) {
      return false;
    }
    this.averageSubmissionMs =
      this.averageSubmissionMs === undefined
        ? durationMs
        : this.averageSubmissionMs * (1 - SAMPLE_SMOOTHING) +
          durationMs * SAMPLE_SMOOTHING;

    if (this.averageSubmissionMs > OVERLOAD_THRESHOLD_MS) {
      this.overloadSamples += 1;
      this.healthySamples = 0;
      if (
        this.overloadSamples >= OVERLOAD_SAMPLE_LIMIT &&
        this.qualityValue !== "minimum"
      ) {
        this.qualityValue = lowerQuality(this.qualityValue);
        this.overloadSamples = 0;
        this.averageSubmissionMs = undefined;
        return true;
      }
      return false;
    }

    if (this.averageSubmissionMs < HEALTHY_THRESHOLD_MS) {
      this.healthySamples += 1;
      this.overloadSamples = 0;
      if (
        this.healthySamples >= HEALTHY_SAMPLE_LIMIT &&
        this.qualityValue !== "full"
      ) {
        this.qualityValue = raiseQuality(this.qualityValue);
        this.healthySamples = 0;
        this.averageSubmissionMs = undefined;
        return true;
      }
      return false;
    }

    this.overloadSamples = 0;
    this.healthySamples = 0;
    return false;
  }

  reset(): void {
    this.qualityValue = "full";
    this.averageSubmissionMs = undefined;
    this.overloadSamples = 0;
    this.healthySamples = 0;
  }
}

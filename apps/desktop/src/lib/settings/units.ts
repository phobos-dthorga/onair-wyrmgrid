import type { AltitudeUnit, FuelUnit, SpeedUnit, WeightUnit } from "./types";

export type PresentedMeasurement = {
  value?: number;
  unit: string;
  digits: number;
};

const FEET_TO_METRES = 0.3048;
const KNOTS_TO_MILES_PER_HOUR = 1.150779448;
const KNOTS_TO_KILOMETRES_PER_HOUR = 1.852;
const KNOTS_TO_METRES_PER_SECOND = 0.514444444;
const POUNDS_TO_KILOGRAMS = 0.45359237;
const US_GALLONS_TO_IMPERIAL_GALLONS = 0.832674185;
const US_GALLONS_TO_LITRES = 3.785411784;

export function presentAltitude(
  feet: number | undefined,
  unit: AltitudeUnit,
): PresentedMeasurement {
  return unit === "metres"
    ? measurement(scale(feet, FEET_TO_METRES), "m", 0)
    : measurement(feet, "ft", 0);
}

export function presentSpeed(
  knots: number | undefined,
  unit: SpeedUnit,
): PresentedMeasurement {
  switch (unit) {
    case "miles_per_hour":
      return measurement(scale(knots, KNOTS_TO_MILES_PER_HOUR), "mph", 0);
    case "kilometres_per_hour":
      return measurement(scale(knots, KNOTS_TO_KILOMETRES_PER_HOUR), "km/h", 0);
    case "metres_per_second":
      return measurement(scale(knots, KNOTS_TO_METRES_PER_SECOND), "m/s", 1);
    default:
      return measurement(knots, "kt", 0);
  }
}

export function presentWeight(
  pounds: number | undefined,
  unit: WeightUnit,
): PresentedMeasurement {
  return unit === "kilograms"
    ? measurement(scale(pounds, POUNDS_TO_KILOGRAMS), "kg", 0)
    : measurement(pounds, "lb", 0);
}

export function presentFuel(
  weightPounds: number | undefined,
  volumeUsGallons: number | undefined,
  unit: FuelUnit,
): PresentedMeasurement {
  switch (unit) {
    case "kilograms":
      return measurement(scale(weightPounds, POUNDS_TO_KILOGRAMS), "kg", 0);
    case "us_gallons":
      return measurement(volumeUsGallons, "US gal", 1);
    case "imperial_gallons":
      return measurement(
        scale(volumeUsGallons, US_GALLONS_TO_IMPERIAL_GALLONS),
        "imp gal",
        1,
      );
    case "litres":
      return measurement(scale(volumeUsGallons, US_GALLONS_TO_LITRES), "L", 1);
    default:
      return measurement(weightPounds, "lb", 0);
  }
}

function scale(value: number | undefined, factor: number): number | undefined {
  return value === undefined || !Number.isFinite(value)
    ? undefined
    : value * factor;
}

function measurement(
  value: number | undefined,
  unit: string,
  digits: number,
): PresentedMeasurement {
  return {
    value: value !== undefined && Number.isFinite(value) ? value : undefined,
    unit,
    digits,
  };
}

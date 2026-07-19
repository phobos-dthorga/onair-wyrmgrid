import { describe, expect, it } from "vitest";
import {
  generateWeatherVolumeDensity,
  generateWeatherVolumeDensityAsync,
} from "./volumeDensity";

function mean(values: readonly number[]): number {
  return values.reduce((total, value) => total + value, 0) / values.length;
}

describe("weather volume density", () => {
  it("is deterministic for one seed and distinct across seeds", () => {
    const first = generateWeatherVolumeDensity(12, 42);
    const repeated = generateWeatherVolumeDensity(12, 42);
    const different = generateWeatherVolumeDensity(12, 43);

    expect(first).toEqual(repeated);
    expect(first).not.toEqual(different);
    expect(first).toHaveLength(12 ** 3);
  });

  it("keeps every texture face empty before building cloud density", () => {
    const size = 16;
    const density = generateWeatherVolumeDensity(size, 7);
    const boundary: number[] = [];
    const centre: number[] = [];
    for (let z = 0; z < size; z += 1) {
      for (let y = 0; y < size; y += 1) {
        for (let x = 0; x < size; x += 1) {
          const value = density[z * size * size + y * size + x];
          if (
            x === 0 ||
            y === 0 ||
            z === 0 ||
            x === size - 1 ||
            y === size - 1 ||
            z === size - 1
          ) {
            boundary.push(value);
          }
          if (x >= 6 && x <= 9 && y >= 6 && y <= 9 && z >= 6 && z <= 9) {
            centre.push(value);
          }
        }
      }
    }

    expect(boundary.every((value) => value === 0)).toBe(true);
    expect(mean(centre)).toBeGreaterThan(0);
    expect(Math.max(...density)).toBeLessThanOrEqual(255);
  });

  it("builds a broad asymmetric multi-lobed body", () => {
    const size = 24;
    const density = generateWeatherVolumeDensity(size, 91);
    const occupied: Array<{ x: number; y: number; z: number }> = [];
    for (let z = 0; z < size; z += 1) {
      for (let y = 0; y < size; y += 1) {
        for (let x = 0; x < size; x += 1) {
          if (density[z * size * size + y * size + x] >= 40) {
            occupied.push({ x, y, z });
          }
        }
      }
    }

    const span = (axis: "x" | "y" | "z") => {
      const values = occupied.map((point) => point[axis]);
      return Math.max(...values) - Math.min(...values) + 1;
    };
    expect(occupied.length).toBeGreaterThan(size ** 3 * 0.04);
    expect(span("x") / span("y")).toBeLessThan(1.7);
    expect(span("z") / span("y")).toBeGreaterThan(0.65);

    let mirroredDifference = 0;
    for (let z = 0; z < size; z += 1) {
      for (let y = 0; y < size; y += 1) {
        for (let x = 0; x < size / 2; x += 1) {
          const left = density[z * size * size + y * size + x];
          const right = density[z * size * size + y * size + (size - 1 - x)];
          mirroredDifference += Math.abs(left - right);
        }
      }
    }
    expect(mirroredDifference).toBeGreaterThan(10_000);
  });

  it("rejects unbounded texture allocations", () => {
    expect(() => generateWeatherVolumeDensity(3)).toThrow(RangeError);
    expect(() => generateWeatherVolumeDensity(129)).toThrow(RangeError);
    expect(() => generateWeatherVolumeDensity(8.5)).toThrow(RangeError);
  });

  it("can yield during initialization without changing the density field", async () => {
    let yields = 0;
    const asynchronous = await generateWeatherVolumeDensityAsync(
      12,
      51,
      async () => {
        yields += 1;
      },
      3,
    );

    expect(asynchronous).toEqual(generateWeatherVolumeDensity(12, 51));
    expect(yields).toBe(3);
  });
});

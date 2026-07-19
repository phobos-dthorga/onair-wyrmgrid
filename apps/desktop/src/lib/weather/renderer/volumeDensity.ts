export const WEATHER_VOLUME_TEXTURE_SIZE = 80;
const DEFAULT_SLICES_PER_YIELD = 5;

function clampUnit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function smooth(value: number): number {
  return value * value * (3 - 2 * value);
}

function hashLattice(x: number, y: number, z: number, seed: number): number {
  let value = seed ^ Math.imul(x, 374_761_393);
  value = Math.imul(value ^ Math.imul(y, 668_265_263), 1_274_126_177);
  value = Math.imul(value ^ Math.imul(z, 2_147_483_647), 2_246_822_519);
  value ^= value >>> 13;
  return (value >>> 0) / 4_294_967_295;
}

function valueNoise(x: number, y: number, z: number, seed: number): number {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const z0 = Math.floor(z);
  const tx = smooth(x - x0);
  const ty = smooth(y - y0);
  const tz = smooth(z - z0);
  const sample = (dx: number, dy: number, dz: number) =>
    hashLattice(x0 + dx, y0 + dy, z0 + dz, seed);
  const mix = (left: number, right: number, amount: number) =>
    left + (right - left) * amount;
  const low = mix(
    mix(sample(0, 0, 0), sample(1, 0, 0), tx),
    mix(sample(0, 1, 0), sample(1, 1, 0), tx),
    ty,
  );
  const high = mix(
    mix(sample(0, 0, 1), sample(1, 0, 1), tx),
    mix(sample(0, 1, 1), sample(1, 1, 1), tx),
    ty,
  );
  return mix(low, high, tz);
}

function fractalNoise(x: number, y: number, z: number, seed: number): number {
  let amplitude = 0.58;
  let frequency = 2.1;
  let total = 0;
  let normalizer = 0;
  for (let octave = 0; octave < 4; octave += 1) {
    total +=
      valueNoise(
        x * frequency,
        y * frequency,
        z * frequency,
        seed + octave * 1_013,
      ) * amplitude;
    normalizer += amplitude;
    amplitude *= 0.52;
    frequency *= 2.03;
  }
  return total / normalizer;
}

type DensityLobe = {
  centre: readonly [number, number, number];
  radius: readonly [number, number, number];
};

const DENSITY_LOBES: readonly DensityLobe[] = [
  { centre: [-0.08, -0.08, 0], radius: [0.82, 0.62, 0.72] },
  { centre: [-0.48, 0.02, 0.08], radius: [0.56, 0.48, 0.55] },
  { centre: [0.42, 0.08, -0.09], radius: [0.6, 0.5, 0.58] },
  { centre: [-0.12, 0.38, -0.05], radius: [0.48, 0.44, 0.5] },
  { centre: [0.24, -0.31, 0.12], radius: [0.52, 0.4, 0.46] },
];

function lobeEnvelope(
  x: number,
  y: number,
  z: number,
  lobe: DensityLobe,
): number {
  const dx = (x - lobe.centre[0]) / lobe.radius[0];
  const dy = (y - lobe.centre[1]) / lobe.radius[1];
  const dz = (z - lobe.centre[2]) / lobe.radius[2];
  const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
  return smooth(clampUnit((1 - distance) / 0.32));
}

function cloudEnvelope(x: number, y: number, z: number): number {
  let envelope = 0;
  for (const lobe of DENSITY_LOBES) {
    envelope = Math.max(envelope, lobeEnvelope(x, y, z, lobe));
  }
  return envelope;
}

function validateSize(size: number): void {
  if (!Number.isInteger(size) || size < 4 || size > 128) {
    throw new RangeError(
      "Weather volume texture size must be an integer in 4..128",
    );
  }
}

function fillDensitySlice(
  data: Uint8Array,
  size: number,
  z: number,
  seed: number,
): void {
  const normalizedZ = (z + 0.5) / size;
  for (let y = 0; y < size; y += 1) {
    const normalizedY = (y + 0.5) / size;
    for (let x = 0; x < size; x += 1) {
      const normalizedX = (x + 0.5) / size;
      const localX = (normalizedX - 0.5) * 2;
      const localY = (normalizedY - 0.5) * 2;
      const localZ = (normalizedZ - 0.5) * 2;
      const envelope = cloudEnvelope(localX, localY, localZ);
      const distanceToTextureFace = Math.min(
        x,
        y,
        z,
        size - 1 - x,
        size - 1 - y,
        size - 1 - z,
      );
      const faceTaper = smooth(
        clampUnit(distanceToTextureFace / Math.max(1, size * 0.1)),
      );
      const base = fractalNoise(normalizedX, normalizedY, normalizedZ, seed);
      const billow = smooth(clampUnit((base - 0.2) / 0.68));
      const index = z * size * size + y * size + x;
      data[index] = Math.round(255 * billow * envelope * faceTaper * faceTaper);
    }
  }
}

/** Creates a repeatable edge-tapered density field; no provider data enters it. */
export function generateWeatherVolumeDensity(
  size = WEATHER_VOLUME_TEXTURE_SIZE,
  seed = 0x5759524d,
): Uint8Array {
  validateSize(size);
  const data = new Uint8Array(size * size * size);
  for (let z = 0; z < size; z += 1) {
    fillDensitySlice(data, size, z, seed);
  }
  return data;
}

export async function generateWeatherVolumeDensityAsync(
  size = WEATHER_VOLUME_TEXTURE_SIZE,
  seed = 0x5759524d,
  yieldControl: () => Promise<void> = () =>
    new Promise((resolve) => setTimeout(resolve, 0)),
  slicesPerYield = DEFAULT_SLICES_PER_YIELD,
): Promise<Uint8Array> {
  validateSize(size);
  if (!Number.isInteger(slicesPerYield) || slicesPerYield < 1) {
    throw new RangeError("Weather volume slices per yield must be positive");
  }
  const data = new Uint8Array(size * size * size);
  for (let z = 0; z < size; z += 1) {
    fillDensitySlice(data, size, z, seed);
    if ((z + 1) % slicesPerYield === 0 && z + 1 < size) {
      await yieldControl();
    }
  }
  return data;
}

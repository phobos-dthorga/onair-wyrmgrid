export const PRECIPITATION_FIELD_TOP = 12;
export const PRECIPITATION_FIELD_HEIGHT = 108;
export const PRECIPITATION_FIELD_WIDTH = 118;

function clampUnit(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function smooth(value: number): number {
  const bounded = clampUnit(value);
  return bounded * bounded * (3 - 2 * bounded);
}

export function precipitationVerticalPosition(
  seedY: number,
  seconds: number,
  speed: number,
): number {
  const wrapped =
    (((seedY + seconds * speed) % PRECIPITATION_FIELD_HEIGHT) +
      PRECIPITATION_FIELD_HEIGHT) %
    PRECIPITATION_FIELD_HEIGHT;
  return PRECIPITATION_FIELD_TOP + wrapped;
}

export function precipitationParticleTaper(x: number, y: number): number {
  const halfWidth = PRECIPITATION_FIELD_WIDTH / 2;
  const horizontal = smooth(
    (halfWidth - Math.abs(x)) / (PRECIPITATION_FIELD_WIDTH * 0.18),
  );
  const progress = clampUnit(
    (y - PRECIPITATION_FIELD_TOP) / PRECIPITATION_FIELD_HEIGHT,
  );
  const upper = smooth(progress / 0.09);
  const lower = smooth((1 - progress) / 0.2);
  return horizontal * upper * lower;
}

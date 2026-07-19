const SURFACE_SAMPLE_DIRECTIONS = [
  [1, 0],
  [-1, 0],
  [0, 1],
  [0, -1],
  [Math.SQRT1_2, Math.SQRT1_2],
  [Math.SQRT1_2, -Math.SQRT1_2],
  [-Math.SQRT1_2, Math.SQRT1_2],
  [-Math.SQRT1_2, -Math.SQRT1_2],
] as const;

export function weatherVisualSurfaceVisibility(
  centreVisibility: number,
  centreX: number,
  centreY: number,
  radius: number,
  visibilityAt: (x: number, y: number) => number,
): number {
  if (centreVisibility <= 0 || radius <= 0) return 0;
  let visibility = centreVisibility;
  for (const [directionX, directionY] of SURFACE_SAMPLE_DIRECTIONS) {
    visibility = Math.min(
      visibility,
      visibilityAt(
        centreX + directionX * radius,
        centreY + directionY * radius,
      ),
    );
    if (visibility <= 0) return 0;
  }
  return visibility;
}

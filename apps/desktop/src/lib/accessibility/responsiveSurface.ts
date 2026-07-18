export type SurfaceBounds = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export type SurfaceReaction = {
  shiftX: number;
  shiftY: number;
  glowX: number;
  glowY: number;
};

const MAX_SHIFT_PX = 1.4;
const GLOW_TRAVEL_PERCENT = 35;

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

export function surfaceReaction(
  clientX: number,
  clientY: number,
  bounds: SurfaceBounds,
): SurfaceReaction {
  if (bounds.width <= 0 || bounds.height <= 0) {
    return { shiftX: 0, shiftY: 0, glowX: 50, glowY: 50 };
  }

  const horizontal = clamp(
    ((clientX - bounds.left) / bounds.width) * 2 - 1,
    -1,
    1,
  );
  const vertical = clamp(
    ((clientY - bounds.top) / bounds.height) * 2 - 1,
    -1,
    1,
  );
  return {
    shiftX: horizontal * MAX_SHIFT_PX,
    shiftY: vertical * MAX_SHIFT_PX,
    glowX: 50 + horizontal * GLOW_TRAVEL_PERCENT,
    glowY: 50 + vertical * GLOW_TRAVEL_PERCENT,
  };
}

function clearReaction(node: HTMLElement): void {
  node.style.removeProperty("--surface-shift-x");
  node.style.removeProperty("--surface-shift-y");
  node.style.removeProperty("--surface-glow-x");
  node.style.removeProperty("--surface-glow-y");
}

/**
 * Delegates pointer tracking for every responsive surface in a UI subtree.
 * New cards only need the shared class; they do not each need their own action.
 */
export function responsiveSurfaceGroup(
  node: HTMLElement,
  options: { enabled: boolean },
): { update: (next: { enabled: boolean }) => void; destroy: () => void } {
  let enabled = options.enabled;
  let activeSurface: HTMLElement | null = null;
  let animationFrame: number | undefined;
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  function reset(): void {
    if (animationFrame !== undefined) {
      window.cancelAnimationFrame(animationFrame);
      animationFrame = undefined;
    }
    if (activeSurface) clearReaction(activeSurface);
    activeSurface = null;
  }

  function handlePointerMove(event: PointerEvent): void {
    const target = event.target;
    const surface =
      target instanceof Element
        ? target.closest<HTMLElement>(".responsive-surface")
        : null;
    if (
      !enabled ||
      reducedMotion.matches ||
      event.pointerType === "touch" ||
      event.pointerType === "pen" ||
      !surface ||
      !node.contains(surface)
    ) {
      reset();
      return;
    }
    if (activeSurface && activeSurface !== surface) reset();
    activeSurface = surface;
    if (animationFrame !== undefined) {
      window.cancelAnimationFrame(animationFrame);
    }
    const reaction = surfaceReaction(
      event.clientX,
      event.clientY,
      surface.getBoundingClientRect(),
    );
    animationFrame = window.requestAnimationFrame(() => {
      surface.style.setProperty("--surface-shift-x", `${reaction.shiftX}px`);
      surface.style.setProperty("--surface-shift-y", `${reaction.shiftY}px`);
      surface.style.setProperty("--surface-glow-x", `${reaction.glowX}%`);
      surface.style.setProperty("--surface-glow-y", `${reaction.glowY}%`);
      animationFrame = undefined;
    });
  }

  node.addEventListener("pointermove", handlePointerMove);
  node.addEventListener("pointerleave", reset);
  reducedMotion.addEventListener("change", reset);

  return {
    update(next): void {
      enabled = next.enabled;
      if (!enabled) reset();
    },
    destroy(): void {
      reset();
      node.removeEventListener("pointermove", handlePointerMove);
      node.removeEventListener("pointerleave", reset);
      reducedMotion.removeEventListener("change", reset);
    },
  };
}

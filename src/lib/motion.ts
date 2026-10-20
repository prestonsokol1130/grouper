import { useSyncExternalStore } from "react";

/** One timed transition: a duration and a CSS easing. */
export type Timing = {
  readonly durationMs: number;
  readonly easing: string;
};

/** An open/close style from DESIGN.md section 10. */
export type OpenCloseStyle = "genie" | "scale" | "smooth";

/** How a window opens (the timing) and closes in one {@link OpenCloseStyle}. */
export type OpenClosePreset = Timing & {
  readonly label: string;
  readonly open: string;
  readonly close: string;
  /** The close: quicker, accelerating away (Fluent "exit"). */
  readonly closing: Timing;
  /** Slide distance in px (Windows smooth only). */
  readonly slidePx?: number;
};

/**
 * Windows 11 (Fluent 2) motion: what appears decelerates into place, what
 * leaves accelerates away. Changed per Preston's feedback, 2026-10-20: the
 * old 500ms Genie on `cubic-bezier(0,0,0,1)` felt choppy and slow.
 */
export const FLUENT_DECELERATE = "cubic-bezier(0.1, 0.9, 0.2, 1)";
export const FLUENT_ACCELERATE = "cubic-bezier(0.7, 0, 1, 0.5)";

/** Open/close styles, DESIGN.md section 10. Genie is the default. */
export const OPEN_CLOSE: Readonly<Record<OpenCloseStyle, OpenClosePreset>> = {
  genie: {
    label: "Genie",
    open: "Pours out of its origin point, tall and narrow first, then widens into shape",
    close: "Shrinks and drops back into the origin point",
    durationMs: 300,
    easing: FLUENT_DECELERATE,
    closing: { durationMs: 180, easing: FLUENT_ACCELERATE },
  },
  scale: {
    label: "Scale",
    open: "Grows from the origin point while fading in",
    close: "Shrinks back into it",
    durationMs: 250,
    easing: FLUENT_DECELERATE,
    closing: { durationMs: 160, easing: FLUENT_ACCELERATE },
  },
  smooth: {
    label: "Windows smooth",
    open: "Fades in and slides up 12px",
    close: "Fades out and slides down",
    durationMs: 200,
    easing: FLUENT_DECELERATE,
    closing: { durationMs: 150, easing: FLUENT_ACCELERATE },
    slidePx: 12,
  },
};

/** The default open/close style. */
export const DEFAULT_OPEN_CLOSE: OpenCloseStyle = "genie";

/** With Windows "Animation effects" off every style is this fade. */
export const REDUCED_FADE: Timing = { durationMs: 120, easing: "linear" };

/** Hover lift and grow, DESIGN.md section 11. */
export const HOVER_LIFT = {
  durationMs: 150,
  easing: "ease-out",
  liftPx: 3,
  scale: 1.08,
  shadow: "0 8px 16px rgba(0,0,0,0.25)",
} as const;

/** Pressed: scales to 0.95 over 80ms. */
export const PRESS = { durationMs: 80, easing: FLUENT_DECELERATE, scale: 0.95 } as const;

/** Launchpad and flyout page swipe. */
export const PAGE_SWIPE: Timing = { durationMs: 250, easing: "ease-out" };

/** The drop target's scale while an app is dragged over it. */
export const DROP_TARGET = { scale: 1.15 } as const;

/** The open timing for `style`, honouring reduced motion. */
export function openCloseTiming(style: OpenCloseStyle, reducedMotion: boolean): Timing {
  return reducedMotion ? REDUCED_FADE : OPEN_CLOSE[style];
}

/** The close timing for `style`, honouring reduced motion. */
export function closeTiming(style: OpenCloseStyle, reducedMotion: boolean): Timing {
  return reducedMotion ? REDUCED_FADE : OPEN_CLOSE[style].closing;
}

/**
 * The curve to give an animation played in reverse so that it moves like
 * `easing` played forwards. A reversed animation runs its curve backwards
 * in time, so `cubic-bezier(x1, y1, x2, y2)` becomes
 * `cubic-bezier(1 - x2, 1 - y2, 1 - x1, 1 - y1)`. Anything else is
 * returned as it is (`linear` is its own mirror).
 */
export function reversedEasing(easing: string): string {
  const match = /^cubic-bezier\(([^)]*)\)$/.exec(easing.trim());
  const points = match?.[1]?.split(",").map((n) => Number(n.trim()));
  if (points?.length !== 4 || points.some((n) => !Number.isFinite(n))) return easing;
  const [x1, y1, x2, y2] = points as [number, number, number, number];
  const flip = (n: number) => Number((1 - n).toFixed(4));
  return `cubic-bezier(${flip(x2)}, ${flip(y2)}, ${flip(x1)}, ${flip(y1)})`;
}

const QUERY = "(prefers-reduced-motion: reduce)";

function subscribe(onChange: () => void): () => void {
  if (typeof window.matchMedia !== "function") return () => undefined;
  const media = window.matchMedia(QUERY);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

function snapshot(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia(QUERY).matches;
}

/**
 * True while Windows "Animation effects" is off (WebView2 reports it as
 * prefers-reduced-motion). Updates live.
 */
export function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, snapshot, () => false);
}

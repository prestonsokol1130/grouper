import { useSyncExternalStore } from "react";

/** One timed transition: a duration and a CSS easing. */
export type Timing = {
  readonly durationMs: number;
  readonly easing: string;
};

/** An open/close style from DESIGN.md section 10. */
export type OpenCloseStyle = "genie" | "scale" | "smooth";

/** How a window opens and closes in one {@link OpenCloseStyle}. */
export type OpenClosePreset = Timing & {
  readonly label: string;
  readonly open: string;
  readonly close: string;
  /** Slide distance in px (Windows smooth only). */
  readonly slidePx?: number;
};

/**
 * Fluent's "decelerate" curve. DESIGN.md gives it for Windows smooth and
 * names no curve for Genie, Scale or the press, so they use it too.
 */
const FLUENT_DECELERATE = "cubic-bezier(0,0,0,1)";

/** Open/close styles, DESIGN.md section 10. Genie is the default. */
export const OPEN_CLOSE: Readonly<Record<OpenCloseStyle, OpenClosePreset>> = {
  genie: {
    label: "Genie",
    open: "Pours out of its origin point and un-pinches into shape",
    close: "Squeezes and is sucked into the origin point",
    durationMs: 500,
    easing: FLUENT_DECELERATE,
  },
  scale: {
    label: "Scale",
    open: "Grows from the origin point while fading in",
    close: "Shrinks back into it",
    durationMs: 300,
    easing: FLUENT_DECELERATE,
  },
  smooth: {
    label: "Windows smooth",
    open: "Fades in and slides up 12px",
    close: "Fades out and slides down",
    durationMs: 200,
    easing: FLUENT_DECELERATE,
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

/** Launchpad page swipe. */
export const PAGE_SWIPE: Timing = { durationMs: 250, easing: "ease-out" };

/** The drop target's scale while an app is dragged over it. */
export const DROP_TARGET = { scale: 1.15 } as const;

/** The timing to use for `style`, honouring reduced motion. */
export function openCloseTiming(style: OpenCloseStyle, reducedMotion: boolean): Timing {
  return reducedMotion ? REDUCED_FADE : OPEN_CLOSE[style];
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

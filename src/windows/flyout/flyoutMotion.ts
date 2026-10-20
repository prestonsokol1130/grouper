import type { CSSProperties } from "react";
import type { FlyoutOrigin } from "../../bindings";
import {
  DEFAULT_OPEN_CLOSE,
  OPEN_CLOSE,
  openCloseTiming,
  type OpenCloseStyle,
} from "../../lib/motion";

/**
 * The flyout's open and close animations (DESIGN.md section 10). The
 * keyframes live in `flyout.css`; this file only toggles the classes that
 * run them and feeds them the motion.ts timing and the origin point. The
 * window is shown by Rust straight away: nothing here delays it.
 */

/** The panel classes that run an animation or keep the panel hidden. */
const PHASES = ["is-opening", "is-closing", "is-hidden"] as const;
type Phase = (typeof PHASES)[number];

/**
 * The open/close style. Settings has no "Animation style" field yet, so
 * this is the default (Genie); the setting replaces it when it lands.
 */
export function useOpenCloseStyle(): OpenCloseStyle {
  return DEFAULT_OPEN_CLOSE;
}

/** `data-motion` on the panel: the style, or `reduced` for the fade. */
export function motionName(style: OpenCloseStyle, reduced: boolean): string {
  return reduced ? "reduced" : style;
}

/** CSS variables for the keyframes: timing, origin and slide distance. */
export function motionVars(
  style: OpenCloseStyle,
  reduced: boolean,
  origin: FlyoutOrigin | undefined,
): CSSProperties {
  const timing = openCloseTiming(style, reduced);
  return {
    "--motion-duration": `${timing.durationMs}ms`,
    "--motion-easing": timing.easing,
    "--motion-slide": `${OPEN_CLOSE.smooth.slidePx ?? 0}px`,
    "--origin-x": `${origin?.x ?? 0}px`,
    "--origin-y": `${origin?.y ?? 0}px`,
  } as CSSProperties;
}

/** Puts `panel` in `phase`, restarting its animation even if unchanged. */
function setPhase(panel: HTMLElement, phase: Phase): void {
  panel.classList.remove(...PHASES);
  // Reading layout makes the browser drop the old animation, so adding
  // the same class again starts it from the beginning.
  void panel.offsetWidth;
  panel.classList.add(phase);
}

/** Starts the open animation. */
export function playOpen(panel: HTMLElement): void {
  setPhase(panel, "is-opening");
}

/** Hides the content at once, so the next show starts from nothing. */
export function resetHidden(panel: HTMLElement): void {
  setPhase(panel, "is-hidden");
}

/**
 * Plays the close animation (the open one in reverse) and resolves when
 * it ends, or after `durationMs` and a frame's slack if no end event
 * comes (for example while the window is not painting).
 */
export function playClose(panel: HTMLElement, durationMs: number): Promise<void> {
  setPhase(panel, "is-closing");
  return new Promise((resolve) => {
    const done = () => {
      window.clearTimeout(timer);
      panel.removeEventListener("animationend", onEnd);
      resolve();
    };
    const onEnd = (event: AnimationEvent) => {
      if (event.target === panel) done();
    };
    panel.addEventListener("animationend", onEnd);
    const timer = window.setTimeout(done, durationMs + 20);
  });
}

/**
 * Turning Launchpad pages from the wheel and from a drag. Pure, so the
 * thresholds are unit-tested; the hooks feed in events and timestamps.
 */

/** A wheel delta smaller than this (px) is noise, not a notch. */
const WHEEL_MIN_DELTA = 4;
/**
 * Wheel events closer together than this (ms) belong to one gesture: a
 * notch, or a trackpad swipe with its momentum. One gesture turns one page.
 */
const WHEEL_QUIET_MS = 160;

/** -1 (previous page), 0 (stay) or 1 (next page). */
export type Step = -1 | 0 | 1;

/**
 * Turns wheel events into page steps: one per gesture, so one mouse notch
 * is one page and a trackpad swipe is one page however long its momentum
 * runs. Vertical and horizontal wheels both page; down and right go
 * forward.
 */
export class WheelPager {
  private last = Number.NEGATIVE_INFINITY;
  private turned = false;

  /** The step for a wheel event of `deltaX`/`deltaY` px at `now` ms. */
  step(deltaX: number, deltaY: number, now: number): Step {
    if (now - this.last > WHEEL_QUIET_MS) this.turned = false;
    this.last = now;
    if (this.turned) return 0;
    const delta = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;
    if (Math.abs(delta) < WHEEL_MIN_DELTA) return 0;
    this.turned = true;
    return delta > 0 ? 1 : -1;
  }
}

/** A drag must move this far (px) sideways before it counts as a drag. */
export const DRAG_START_PX = 8;
/** Dragged this share of the page width, a release turns the page. */
const DRAG_TURN_SHARE = 0.25;
/** A flick this fast (px/ms) turns the page however short it was. */
const FLICK_SPEED = 0.5;
/** ...as long as it moved at least this far (px). */
const FLICK_MIN_PX = 24;
/** Past the first or last page the drag follows at this rate. */
export const EDGE_RESISTANCE = 0.3;

/**
 * The step when a drag of `dx` px (positive is to the right) ends,
 * moving at `velocity` px/ms over its last moments, on a page `width` px
 * wide. Dragging to the left shows the next page.
 */
export function dragStep(dx: number, velocity: number, width: number): Step {
  const far = Math.abs(dx) >= width * DRAG_TURN_SHARE;
  const flick =
    Math.abs(velocity) >= FLICK_SPEED &&
    Math.abs(dx) >= FLICK_MIN_PX &&
    Math.sign(velocity) === Math.sign(dx);
  if (!far && !flick) return 0;
  return dx < 0 ? 1 : -1;
}

/** `page`, kept between the first and last of `count` pages. */
export function clampPage(page: number, count: number): number {
  return Math.min(Math.max(0, page), Math.max(0, count - 1));
}

import { useRef, type MouseEvent, type PointerEvent } from "react";
import { DRAG_START_PX, dragStep, EDGE_RESISTANCE, type Step } from "./paging";

/** How far back (ms) the release velocity is measured. */
const VELOCITY_WINDOW_MS = 80;

type Sample = { x: number; t: number };

type Drag = {
  id: number;
  startX: number;
  startY: number;
  dragging: boolean;
  samples: Sample[];
};

type Options = {
  page: number;
  count: number;
  /** Moves the track `offset` px from the current page, without animating. */
  follow: (offset: number) => void;
  /** The drag ended: turn by `step`, or (0) slide back into place. */
  release: (step: Step) => void;
};

/** The px/ms speed over the last moments of `samples`. */
function velocity(samples: readonly Sample[]): number {
  const last = samples.at(-1);
  if (last === undefined) return 0;
  const first = samples.find((s) => last.t - s.t <= VELOCITY_WINDOW_MS) ?? last;
  const dt = last.t - first.t;
  return dt > 0 ? (last.x - first.x) / dt : 0;
}

/**
 * Pointer drag and swipe paging, for presses off the tiles (a press on a
 * tile picks the tile up, see SortableTile). A press becomes a drag once it moves
 * DRAG_START_PX sideways (more than up or down); the track then follows
 * the pointer (with resistance past the first and last page) and the
 * release turns the page when dragged far enough or flicked. The click
 * that ends a drag is swallowed so it neither launches a tile nor closes
 * the Launchpad.
 */
export function useDragPaging({ page, count, follow, release }: Options) {
  const drag = useRef<Drag | null>(null);
  const swallowClick = useRef(false);

  const offsetFor = (dx: number) => {
    const pastEdge = (page === 0 && dx > 0) || (page >= count - 1 && dx < 0);
    return pastEdge ? dx * EDGE_RESISTANCE : dx;
  };

  const onPointerDown = (event: PointerEvent<HTMLElement>) => {
    swallowClick.current = false;
    drag.current = null;
    if (event.button !== 0 || !event.isPrimary) return;
    // A press on a tile of the page on screen moves the tile instead.
    if (event.target instanceof Element && event.target.closest("[data-sortable]")) return;
    const sample = { x: event.clientX, t: event.timeStamp };
    drag.current = {
      id: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      dragging: false,
      samples: [sample],
    };
  };

  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const current = drag.current;
    if (current === null || current.id !== event.pointerId) return;
    const dx = event.clientX - current.startX;
    if (!current.dragging) {
      const dy = event.clientY - current.startY;
      if (Math.abs(dx) < DRAG_START_PX || Math.abs(dx) <= Math.abs(dy)) return;
      current.dragging = true;
      event.currentTarget.setPointerCapture?.(event.pointerId);
    }
    current.samples.push({ x: event.clientX, t: event.timeStamp });
    if (current.samples.length > 12) current.samples.shift();
    follow(offsetFor(dx));
  };

  const end = (event: PointerEvent<HTMLElement>, cancelled: boolean) => {
    const current = drag.current;
    if (current === null || current.id !== event.pointerId) return;
    drag.current = null;
    if (!current.dragging) return;
    swallowClick.current = !cancelled;
    current.samples.push({ x: event.clientX, t: event.timeStamp });
    const dx = event.clientX - current.startX;
    const width = event.currentTarget.clientWidth || window.innerWidth;
    release(cancelled ? 0 : dragStep(dx, velocity(current.samples), width));
  };

  const onClickCapture = (event: MouseEvent<HTMLElement>) => {
    if (!swallowClick.current) return;
    swallowClick.current = false;
    event.stopPropagation();
    event.preventDefault();
  };

  return {
    onPointerDown,
    onPointerMove,
    onPointerUp: (event: PointerEvent<HTMLElement>) => end(event, false),
    onPointerCancel: (event: PointerEvent<HTMLElement>) => end(event, true),
    onClickCapture,
  };
}

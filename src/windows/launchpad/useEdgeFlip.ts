import { useCallback, useEffect, useRef } from "react";
import type { Step } from "./paging";

/**
 * How close (px) to the left or right edge of the window a dragged tile
 * turns the page: the Launchpad's smallest side padding, so the zone never
 * covers a tile.
 */
const EDGE_ZONE_PX = 48;

/** How long (ms) a dragged tile rests at an edge before the page turns. */
export const EDGE_FLIP_MS = 600;

/** -1 near the left edge, 1 near the right edge, else 0. */
export function edgeSide(x: number, width: number, zone = EDGE_ZONE_PX): Step {
  if (width <= 0) return 0;
  if (x <= zone) return -1;
  if (x >= width - zone) return 1;
  return 0;
}

/**
 * Turns the page while a dragged tile rests at an edge: after
 * EDGE_FLIP_MS, and again every EDGE_FLIP_MS while it stays there.
 * `track` takes the pointer's x (null when the drag ends or is not a
 * pointer drag); leaving the edge stops the timer.
 */
export function useEdgeFlip(turn: (step: Step) => void): (x: number | null) => void {
  const latest = useRef(turn);
  const side = useRef<Step>(0);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    latest.current = turn;
  });

  const stop = useCallback(() => {
    if (timer.current !== null) clearInterval(timer.current);
    timer.current = null;
    side.current = 0;
  }, []);
  useEffect(() => stop, [stop]);

  return useCallback(
    (x: number | null) => {
      const next = x === null ? 0 : edgeSide(x, window.innerWidth);
      if (next === side.current) return;
      stop();
      if (next === 0) return;
      side.current = next;
      timer.current = setInterval(() => latest.current(next), EDGE_FLIP_MS);
    },
    [stop],
  );
}

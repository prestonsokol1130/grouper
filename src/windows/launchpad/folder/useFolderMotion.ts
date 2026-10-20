import { useCallback, useLayoutEffect, useRef } from "react";
import type { Edge } from "../../../bindings";
import { openCloseTiming, type OpenCloseStyle } from "../../../lib/motion";
import { playClose, playOpen } from "../../flyout/flyoutMotion";

/** A point in client (viewport) px. */
export type Point = { readonly x: number; readonly y: number };

type Rect = { left: number; top: number; right: number; bottom: number };

/**
 * The side of `panel` the folder tile at `origin` is on, for the genie's
 * pinch and the smooth slide: below or above it, else left or right of
 * it, else (a tile under the panel) the nearer of top and bottom.
 */
export function originEdge(origin: Point, panel: Rect): Edge {
  if (origin.y >= panel.bottom) return "bottom";
  if (origin.y <= panel.top) return "top";
  if (origin.x <= panel.left) return "left";
  if (origin.x >= panel.right) return "right";
  return origin.y >= (panel.top + panel.bottom) / 2 ? "bottom" : "top";
}

/** Puts the animation origin at `origin` (client px) and plays the open. */
function openFrom(panel: HTMLElement, origin: Point | null): void {
  const rect = panel.getBoundingClientRect();
  const from = origin ?? { x: rect.left + rect.width / 2, y: rect.bottom };
  panel.style.setProperty("--origin-x", `${from.x - rect.left}px`);
  panel.style.setProperty("--origin-y", `${from.y - rect.top}px`);
  panel.dataset.edge = originEdge(from, rect);
  playOpen(panel);
}

/**
 * The folder panel's open and close (DESIGN.md section 10: "folder open
 * uses the same style as the flyout"). The flyout's keyframes run on the
 * panel with the folder tile's centre as the origin point; with reduced
 * motion they are the 120ms fade. `close` plays the reverse and then
 * calls `onClosed`; a second close while one plays does nothing.
 */
export function useFolderMotion(
  origin: Point | null,
  style: OpenCloseStyle,
  reduced: boolean,
  onClosed: () => void,
) {
  const panel = useRef<HTMLDivElement>(null);
  const closing = useRef(false);
  const latest = useRef(onClosed);
  useLayoutEffect(() => {
    latest.current = onClosed;
  });

  const from = useRef(origin);
  useLayoutEffect(() => {
    if (panel.current !== null) openFrom(panel.current, from.current);
  }, []);

  const close = useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    const element = panel.current;
    const timing = openCloseTiming(style, reduced);
    const played = element ? playClose(element, timing.durationMs) : Promise.resolve();
    void played.then(() => latest.current());
  }, [style, reduced]);
  return { panel, close };
}

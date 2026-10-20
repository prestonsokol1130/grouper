import { useEffect, useRef, useState } from "react";

/** Lists longer than this render only the rows near the viewport. */
const WINDOW_THRESHOLD = 300;

/** Rows rendered above and below the viewport while windowed. */
const OVERSCAN = 8;

/** Viewport height used until the real one is measured (and in tests). */
const FALLBACK_HEIGHT = 480;

/**
 * The rows `[start, end)` to render for a viewport scrolled `top` px and
 * `height` px tall, over `count` rows `rowHeight` px each.
 */
export function visibleRange(
  top: number,
  height: number,
  rowHeight: number,
  count: number,
): { start: number; end: number } {
  if (rowHeight <= 0) return { start: 0, end: count };
  const start = Math.max(0, Math.floor(top / rowHeight) - OVERSCAN);
  const end = Math.min(count, Math.ceil((top + height) / rowHeight) + OVERSCAN);
  return { start, end: Math.max(start, end) };
}

/**
 * Fixed-row-height windowing for a scrolling list. With `count` above
 * {@link WINDOW_THRESHOLD}, only the rows near the viewport are rendered,
 * each placed absolutely at `index * rowHeight`; shorter lists render
 * everything. Put `viewport` and `onScroll` on the scrolling element.
 */
export function useWindowedRows(count: number, rowHeight: number) {
  const viewport = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ top: 0, height: FALLBACK_HEIGHT });
  const windowed = count > WINDOW_THRESHOLD;

  useEffect(() => {
    const element = viewport.current;
    if (element === null || typeof ResizeObserver === "undefined") return;
    // Fires once on observe, so the first real height arrives at once.
    const observer = new ResizeObserver(() =>
      setView({ top: element.scrollTop, height: element.clientHeight || FALLBACK_HEIGHT }),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const range = windowed
    ? visibleRange(view.top, view.height, rowHeight, count)
    : { start: 0, end: count };

  /** Scrolls row `index` into view and renders it at once. */
  const reveal = (index: number) => {
    const element = viewport.current;
    if (element === null) return;
    const rowTop = index * rowHeight;
    const height = element.clientHeight || view.height;
    let top = element.scrollTop;
    if (rowTop < top) top = rowTop;
    else if (rowTop + rowHeight > top + height) top = rowTop + rowHeight - height;
    element.scrollTop = top;
    setView({ top, height });
  };

  return {
    viewport,
    windowed,
    range,
    /** The full list height while windowed. */
    totalHeight: count * rowHeight,
    reveal,
    onScroll: () => {
      const element = viewport.current;
      if (element === null) return;
      setView({ top: element.scrollTop, height: element.clientHeight || FALLBACK_HEIGHT });
    },
  };
}

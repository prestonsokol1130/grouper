import { useRef, useState, type MouseEvent, type PointerEvent } from "react";

/** How far (px) the pointer must travel before a press becomes a drag. */
const DRAG_THRESHOLD = 4;

/** `items` with the one at `from` moved to `to`. */
export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  const next = [...items];
  const [moved] = next.splice(from, 1);
  if (moved === undefined) return next;
  next.splice(Math.max(0, Math.min(to, next.length)), 0, moved);
  return next;
}

/**
 * The slot a row dragged `dy` px from slot `start` lands in, when rows are
 * `step` px apart: the nearest slot, kept inside the list.
 */
export function dropIndex(start: number, dy: number, step: number, count: number): number {
  if (step <= 0 || count <= 0) return start;
  return Math.max(0, Math.min(count - 1, start + Math.round(dy / step)));
}

/** The distance between the tops of two neighbouring list items. */
function rowStep(row: HTMLElement): number {
  const item = row.closest("li") ?? row;
  const siblings = item.parentElement?.children;
  const first = siblings?.[0];
  const second = siblings?.[1];
  if (first && second) {
    const gap = second.getBoundingClientRect().top - first.getBoundingClientRect().top;
    if (gap > 0) return gap;
  }
  return item.getBoundingClientRect().height;
}

type Drag = { id: string; start: number; startY: number; step: number; dy: number; moved: boolean };

/**
 * Pointer drag-to-reorder for a vertical list (plain pointer events, so it
 * also works where the webview's HTML5 drag and drop is taken by Tauri's
 * file drop). Spread `rowProps(id)` on each row. While a row is dragged,
 * `order` is the order it would drop into and `offset` keeps the dragged
 * row under the pointer. A drop that changes the order calls `onReorder`.
 */
export function useDragReorder(ids: string[], onReorder: (ids: string[]) => void) {
  const [drag, setDragState] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  // A drag ends with a click on the row; that click must not select it.
  const swallowClick = useRef(false);

  const setDrag = (next: Drag | null) => {
    dragRef.current = next;
    setDragState(next);
  };
  const target = drag ? dropIndex(drag.start, drag.dy, drag.step, ids.length) : 0;
  const active = drag !== null && drag.moved && ids[drag.start] === drag.id;
  const order = active ? moveItem(ids, drag.start, target) : ids;

  const finish = (commit: boolean) => {
    const current = dragRef.current;
    setDrag(null);
    if (current === null || !current.moved) return;
    swallowClick.current = true;
    if (!commit || ids[current.start] !== current.id) return;
    const to = dropIndex(current.start, current.dy, current.step, ids.length);
    if (to !== current.start) onReorder(moveItem(ids, current.start, to));
  };

  const rowProps = (id: string) => ({
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return;
      const row = event.currentTarget;
      if (typeof row.setPointerCapture === "function") row.setPointerCapture(event.pointerId);
      swallowClick.current = false;
      const start = ids.indexOf(id);
      const step = rowStep(row);
      setDrag({ id, start, startY: event.clientY, step, dy: 0, moved: false });
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const current = dragRef.current;
      if (current === null || current.id !== id) return;
      const dy = event.clientY - current.startY;
      const moved = current.moved || Math.abs(dy) > DRAG_THRESHOLD;
      if (moved) setDrag({ ...current, dy, moved });
    },
    onPointerUp: () => finish(true),
    onPointerCancel: () => finish(false),
    onLostPointerCapture: () => finish(true),
    onClickCapture: (event: MouseEvent<HTMLElement>) => {
      if (!swallowClick.current) return;
      swallowClick.current = false;
      event.preventDefault();
      event.stopPropagation();
    },
  });

  return {
    order,
    draggingId: active ? drag.id : null,
    /** px to shift the dragged row so it stays under the pointer. */
    offset: active ? drag.dy - (target - drag.start) * drag.step : 0,
    rowProps,
    /** Drops the drag without reordering (Escape). */
    cancel: () => finish(false),
  };
}

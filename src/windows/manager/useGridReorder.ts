import { useRef, useState, type MouseEvent, type PointerEvent } from "react";
import { moveItem } from "./useDragReorder";

/** How far (px) the pointer must travel before a press becomes a drag. */
const DRAG_THRESHOLD = 4;

export type Point = { x: number; y: number };

/** The index of the slot whose centre is nearest `point` (0 with none). */
export function nearestSlot(slots: readonly Point[], point: Point): number {
  let best = 0;
  let bestDistance = Number.POSITIVE_INFINITY;
  slots.forEach((slot, index) => {
    const distance = (slot.x - point.x) ** 2 + (slot.y - point.y) ** 2;
    if (distance < bestDistance) {
      best = index;
      bestDistance = distance;
    }
  });
  return best;
}

/** The centres of the cells in `list`, in DOM order. */
function slotsOf(list: Element | null): Point[] {
  return [...(list?.children ?? [])].map((cell) => {
    const rect = cell.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  });
}

type Drag = {
  id: string;
  start: number;
  origin: Point;
  slots: Point[];
  delta: Point;
  moved: boolean;
};

const NO_SHIFT: Point = { x: 0, y: 0 };

/**
 * Pointer drag-to-reorder for a wrapping grid, like `useDragReorder` for
 * the sidebar but in two dimensions. Spread `cellProps(id)` on each cell
 * (the list's direct children) and translate it by `shift(id)`. A press
 * on an element inside `[data-no-drag]` never starts a drag. The pointer
 * is captured only once the press becomes a drag, so a plain click still
 * reaches the tile inside the cell.
 */
export function useGridReorder(ids: string[], onReorder: (ids: string[]) => void) {
  const [drag, setDragState] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  // A drag ends with a click on the cell; that click must not open the app.
  const swallowClick = useRef(false);

  const setDrag = (next: Drag | null) => {
    dragRef.current = next;
    setDragState(next);
  };
  const active = drag !== null && drag.moved && ids[drag.start] === drag.id;
  const target = active ? nearestSlot(drag.slots, add(drag.slots[drag.start], drag.delta)) : 0;
  const order = active ? moveItem(ids, drag.start, target) : ids;

  const shift = (id: string): Point => {
    if (!active) return NO_SHIFT;
    const from = ids.indexOf(id);
    if (from === drag.start) return drag.delta;
    const here = drag.slots[from];
    const there = drag.slots[order.indexOf(id)];
    if (here === undefined || there === undefined) return NO_SHIFT;
    return { x: there.x - here.x, y: there.y - here.y };
  };

  const finish = (commit: boolean) => {
    const current = dragRef.current;
    setDrag(null);
    if (current === null || !current.moved) return;
    swallowClick.current = true;
    if (!commit || ids[current.start] !== current.id) return;
    const to = nearestSlot(current.slots, add(current.slots[current.start], current.delta));
    if (to !== current.start) onReorder(moveItem(ids, current.start, to));
  };

  const cellProps = (id: string) => ({
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return;
      if (event.target instanceof Element && event.target.closest("[data-no-drag]")) return;
      swallowClick.current = false;
      const origin = { x: event.clientX, y: event.clientY };
      const slots = slotsOf(event.currentTarget.parentElement);
      setDrag({ id, start: ids.indexOf(id), origin, slots, delta: NO_SHIFT, moved: false });
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const current = dragRef.current;
      if (current === null || current.id !== id) return;
      const delta = { x: event.clientX - current.origin.x, y: event.clientY - current.origin.y };
      const moved = current.moved || Math.hypot(delta.x, delta.y) > DRAG_THRESHOLD;
      if (!moved) return;
      const cell = event.currentTarget;
      if (!current.moved && typeof cell.setPointerCapture === "function") {
        cell.setPointerCapture(event.pointerId);
      }
      setDrag({ ...current, delta, moved });
    },
    onPointerUp: () => finish(true),
    onPointerCancel: () => finish(false),
    onLostPointerCapture: () => finish(false),
    onClickCapture: (event: MouseEvent<HTMLElement>) => {
      if (!swallowClick.current) return;
      swallowClick.current = false;
      event.preventDefault();
      event.stopPropagation();
    },
  });

  return {
    draggingId: active ? drag.id : null,
    shift,
    cellProps,
    /** Drops the drag without reordering (Escape). */
    cancel: () => finish(false),
  };
}

function add(point: Point | undefined, delta: Point): Point {
  return { x: (point?.x ?? 0) + delta.x, y: (point?.y ?? 0) + delta.y };
}

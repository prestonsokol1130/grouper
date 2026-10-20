import type { DragEndEvent, DragMoveEvent, DragStartEvent, UniqueIdentifier } from "@dnd-kit/core";
import { useMemo, useState } from "react";
import type { LaunchpadItem } from "../../bindings";
import { findItem, previewPages } from "../../lib/layoutMap";
import type { Position } from "../../lib/layoutOps";
import { itemKey } from "./launchpadGrid";
import { useEdgeFlip } from "./useEdgeFlip";
import type { Pager } from "./usePager";

/** A tile being dragged: its key, the slot it would drop into, the press x. */
export type Drag = {
  readonly id: string;
  readonly target: Position;
  /** Where a pointer drag was pressed (null for a keyboard drag). */
  readonly startX: number | null;
};

/** Holds the current drag; the Launchpad root owns it, as the pager does. */
export function useDragState() {
  return useState<Drag | null>(null);
}

/** Every slot a page has: a page the drag turns to puts the tile at its end. */
const END = Number.MAX_SAFE_INTEGER;

type Options = {
  /** The pages on screen, with the empty page a drag may open at the end. */
  shown: readonly (readonly LaunchpadItem[])[];
  perPage: number;
  drag: Drag | null;
  setDrag: (drag: Drag | null) => void;
  pager: Pager;
  /** Saves the drop of tile `id` on screen slot `drop`. */
  commit: (id: string, drop: Position) => void;
};

/** The pointer's x when `event` started a pointer drag, else null. */
function pressX(event: Event): number | null {
  return "clientX" in event && typeof event.clientX === "number" ? event.clientX : null;
}

/**
 * Reordering by drag and drop. The tile can only drop onto the page on
 * screen: when the page turns (a rest at the edge, the wheel) the tile
 * moves to the end of the new page and the drop slot goes with it. The
 * pages returned are what to draw: `shown` with the tile in its slot.
 */
export function useReorder({ shown, perPage, drag, setDrag, pager, commit }: Options) {
  const track = useEdgeFlip((step) => pager.turn(step));

  // The page turned during the drag: the tile follows it to its end. Back
  // on the page it started from, it is back in the slot it was lifted from.
  const current = useMemo(
    () =>
      drag === null || drag.target.page === pager.page
        ? drag
        : { ...drag, target: { page: pager.page, index: END } },
    [drag, pager.page],
  );

  const pages = useMemo(
    () =>
      current === null ? shown : previewPages(shown, itemKey, current.id, current.target, perPage),
    [shown, current, perPage],
  );
  const activeItem = useMemo(() => {
    if (drag === null) return null;
    const at = findItem(shown, itemKey, drag.id);
    return at === null ? null : (shown[at.page]?.[at.index] ?? null);
  }, [shown, drag]);

  const end = () => {
    track(null);
    setDrag(null);
  };

  const onDragStart = ({ active, activatorEvent }: DragStartEvent) => {
    const id = String(active.id);
    const at = findItem(shown, itemKey, id);
    if (at === null) return;
    setDrag({ id, target: at, startX: pressX(activatorEvent) });
  };

  const onDragMove = ({ delta }: DragMoveEvent) => {
    if (current?.startX != null) track(current.startX + delta.x);
  };

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const id = String(active.id);
    const page = current?.target.page;
    const items = page === undefined ? undefined : pages[page];
    if (page !== undefined && items !== undefined) {
      const ids: UniqueIdentifier[] = items.map(itemKey);
      const overIndex = over === null ? -1 : ids.indexOf(over.id);
      const index = overIndex >= 0 ? overIndex : ids.indexOf(id);
      if (index >= 0) commit(id, { page, index });
    }
    end();
  };

  const onDragCancel = () => end();

  return {
    pages,
    activeItem,
    handlers: { onDragStart, onDragMove, onDragEnd, onDragCancel },
  };
}

import {
  closestCenter,
  type ClientRect,
  type CollisionDetection,
  type UniqueIdentifier,
} from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";

/** A point in client (viewport) px. */
type Coordinates = { readonly x: number; readonly y: number };

/**
 * Drop intent while a tile is dragged. Over the centre of another tile an
 * app waits to be merged into it (a new folder, or into that folder);
 * anywhere else, including a tile's edges, it reorders as before.
 */

/** How long (ms) a dragged app rests over a tile's centre before it merges. */
export const MERGE_HOLD_MS = 400;

/** The share of a tile's width and height, around its middle, that merges. */
const MERGE_CENTRE = 0.5;

/** True when `point` is in the middle MERGE_CENTRE of `rect`, both ways. */
export function inCentre(rect: ClientRect, point: Coordinates, share = MERGE_CENTRE): boolean {
  const insetX = (rect.width * (1 - share)) / 2;
  const insetY = (rect.height * (1 - share)) / 2;
  return (
    point.x >= rect.left + insetX &&
    point.x <= rect.right - insetX &&
    point.y >= rect.top + insetY &&
    point.y <= rect.bottom - insetY
  );
}

/**
 * The tile whose centre `point` is over, as drawn while `activeId` is
 * sorted over `overId`. Tiles are measured where they sit in the layout
 * (`rects`, keyed by id); the sorting draws slot `j` with the tile at `j`
 * of `ids` moved like `arrayMove(ids, active, over)`. Null over the
 * dragged tile's own slot, a gap or an edge.
 */
export function centreTarget(
  point: Coordinates,
  ids: readonly UniqueIdentifier[],
  activeId: UniqueIdentifier,
  overId: UniqueIdentifier | null,
  rects: ReadonlyMap<UniqueIdentifier, ClientRect>,
): UniqueIdentifier | null {
  const from = ids.indexOf(activeId);
  const to = overId === null ? from : ids.indexOf(overId);
  const drawn = from < 0 || to < 0 ? [...ids] : arrayMove([...ids], from, to);
  for (const [slot, id] of ids.entries()) {
    const rect = rects.get(id);
    if (rect !== undefined && inCentre(rect, point)) {
      const shown = drawn[slot];
      return shown === undefined || shown === activeId ? null : shown;
    }
  }
  return null;
}

/** The ids, in order, of the sortable list the active tile is in. */
function sortedIds(data: Record<string, unknown> | undefined): readonly UniqueIdentifier[] {
  const sortable = data?.sortable as { items?: UniqueIdentifier[] } | undefined;
  return sortable?.items ?? [];
}

/**
 * How long (ms) a dragged app must rest (within REST_PX) by a new reorder
 * target before the tiles slide for it. Without the pause the tile the
 * app heads for would slide away before the pointer reached its centre
 * (iOS pauses too). A drop before then still lands on that target.
 */
export const REORDER_SETTLE_MS = 150;

/** How far (px) the pointer may drift and still count as resting. */
const REST_PX = 6;

/** Collision detection with drop intent, and what it last saw. */
export type IntentCollision = {
  readonly detect: CollisionDetection;
  /** The tile whose centre the pointer was last over, if it can merge. */
  readonly centre: () => UniqueIdentifier | null;
  /** A reorder target still settling (see REORDER_SETTLE_MS), if any. */
  readonly settling: () => UniqueIdentifier | null;
  /** Forgets the last drag; called when a drag starts. */
  readonly reset: () => void;
};

/**
 * Collision detection with drop intent. With the pointer over another
 * tile's centre, and `canMerge(activeId)`, it records that tile (see
 * `centre`) and keeps the last reorder target, so no tile slides out from
 * under the pointer. Elsewhere it is dnd-kit's closestCenter, except that
 * for an app a new target only takes over once it has held for
 * REORDER_SETTLE_MS (the caller re-renders then; see `settling`). A
 * folder, a keyboard drag and the first target of a drag switch at once.
 */
export function intentCollision(
  canMerge: (activeId: UniqueIdentifier) => boolean,
  now: () => number = () => performance.now(),
): IntentCollision {
  let over: UniqueIdentifier | null = null;
  let centre: UniqueIdentifier | null = null;
  let pending: UniqueIdentifier | null = null;
  let since = 0;
  let anchor: Coordinates = { x: 0, y: 0 };
  const held = () => (over === null ? [] : [{ id: over }]);
  const detect: CollisionDetection = (args) => {
    const point = args.pointerCoordinates;
    const { id, data } = args.active;
    const merges = point !== null && canMerge(id);
    centre = merges
      ? centreTarget(point, sortedIds(data.current), id, over, args.droppableRects)
      : null;
    const hits = closestCenter(args);
    const next = hits[0]?.id ?? null;
    if (centre !== null || next === over) {
      pending = null;
      return held();
    }
    if (merges && over !== null) {
      // The wait restarts while the pointer is still on its way.
      const moved = Math.hypot(point.x - anchor.x, point.y - anchor.y) > REST_PX;
      if (next !== pending || moved) {
        pending = next;
        since = now();
        anchor = point;
      }
      if (now() - since < REORDER_SETTLE_MS) return held();
    }
    pending = null;
    over = next;
    return hits;
  };
  return {
    detect,
    centre: () => centre,
    settling: () => pending,
    reset: () => {
      over = null;
      centre = null;
      pending = null;
    },
  };
}

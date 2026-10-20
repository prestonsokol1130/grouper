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

/** Collision detection with drop intent, and what it last saw. */
export type IntentCollision = {
  readonly detect: CollisionDetection;
  /** The tile whose centre the pointer was last over, if it can merge. */
  readonly centre: () => UniqueIdentifier | null;
  /** Forgets the last drag; called when a drag starts. */
  readonly reset: () => void;
};

/**
 * Collision detection with drop intent. With the pointer over another
 * tile's centre, and `canMerge(activeId)`, it records that tile (see
 * `centre`) and keeps the last reorder target, so no tile slides out from
 * under the pointer. Elsewhere it is dnd-kit's closestCenter.
 */
export function intentCollision(
  canMerge: (activeId: UniqueIdentifier) => boolean,
): IntentCollision {
  let over: UniqueIdentifier | null = null;
  let centre: UniqueIdentifier | null = null;
  const detect: CollisionDetection = (args) => {
    const point = args.pointerCoordinates;
    const { id, data } = args.active;
    centre =
      point === null || !canMerge(id)
        ? null
        : centreTarget(point, sortedIds(data.current), id, over, args.droppableRects);
    if (centre !== null) return over === null ? [] : [{ id: over }];
    const hits = closestCenter(args);
    over = hits[0]?.id ?? null;
    return hits;
  };
  return {
    detect,
    centre: () => centre,
    reset: () => {
      over = null;
      centre = null;
    },
  };
}

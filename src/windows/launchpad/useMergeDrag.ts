import { useState } from "react";
import { intentCollision } from "./mergeIntent";
import { useMergeIntent } from "./useMergeIntent";

/** Only apps merge; a dragged folder always reorders. */
const isAppKey = (id: string | number) => String(id).startsWith("app:");

/**
 * Drop intent for the Launchpad drag: the collision detection to give
 * DndContext, the tile a drop would merge into (`target`, null to
 * reorder) and the calls the drag handlers make. `commit` saves a merge
 * of tile `dragged` into tile `target` (both item keys).
 */
export function useMergeDrag(commit: (dragged: string, target: string) => void) {
  const [collision] = useState(() => intentCollision(isAppKey));
  const intent = useMergeIntent();

  return {
    collision: collision.detect,
    target: intent.target,
    start: () => {
      collision.reset();
      intent.reset();
    },
    /** After each pointer move: the collision detection has just run. */
    move: () => {
      const centre = collision.centre();
      intent.hover(centre === null ? null : String(centre));
    },
    /** On drop: merges and returns true when the drag had merge intent. */
    drop: (dragged: string): boolean => {
      const target = intent.target;
      intent.stop();
      if (target === null || target === dragged) return false;
      commit(dragged, target);
      return true;
    },
    cancel: () => intent.reset(),
  };
}

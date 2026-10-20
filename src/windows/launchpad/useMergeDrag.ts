import type { UniqueIdentifier } from "@dnd-kit/core";
import { useEffect, useRef, useState } from "react";
import { intentCollision, REORDER_SETTLE_MS } from "./mergeIntent";
import { useMergeIntent } from "./useMergeIntent";

/** Only apps merge; a dragged folder always reorders. */
const isAppKey = (id: UniqueIdentifier) => String(id).startsWith("app:");

/**
 * Drop intent for the Launchpad drag: the collision detection to give
 * DndContext, the tile a drop would merge into (`target`, null to
 * reorder) and the calls the drag handlers make. `commit` saves a merge
 * of tile `dragged` into tile `target` (both item keys).
 */
export function useMergeDrag(commit: (dragged: string, target: string) => void) {
  const [collision] = useState(() => intentCollision(isAppKey));
  const intent = useMergeIntent();
  // A re-render runs the collision detection again, so a reorder target
  // that has settled takes over even while the pointer rests.
  const [, setSettled] = useState(0);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const stopSettle = () => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  };
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  return {
    collision: collision.detect,
    target: intent.target,
    start: () => {
      collision.reset();
      intent.reset();
      stopSettle();
    },
    /** After each pointer move: the collision detection has just run. */
    move: () => {
      const centre = collision.centre();
      intent.hover(centre === null ? null : String(centre));
      // Each move restarts the wait, so this fires once the pointer rests.
      stopSettle();
      if (collision.settling() === null) return;
      // A frame past the settle time, so the rerun sees it has passed.
      timer.current = setTimeout(() => setSettled((n) => n + 1), REORDER_SETTLE_MS + 16);
    },
    /** The reorder target a drop lands on, settled or not (null: dnd-kit's). */
    reorderOver: (): UniqueIdentifier | null => collision.settling(),
    /** On drop: merges and returns true when the drag had merge intent. */
    drop: (dragged: string): boolean => {
      const target = intent.target;
      intent.stop();
      stopSettle();
      if (target === null || target === dragged) return false;
      commit(dragged, target);
      return true;
    },
    cancel: () => {
      intent.reset();
      stopSettle();
    },
  };
}

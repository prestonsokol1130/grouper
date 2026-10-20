import { useCallback, useEffect, useRef, useState } from "react";
import { MERGE_HOLD_MS } from "./mergeIntent";

/** The merge intent of a drag, and the calls that drive it. */
export type MergeIntent = {
  /** The tile the dragged app would merge into on drop, else null (reorder). */
  readonly target: string | null;
  /** The pointer is over the centre of tile `id` (null: not over a centre). */
  readonly hover: (id: string | null) => void;
  /** Ends the wait but keeps `target`, for the drop that follows. */
  readonly stop: () => void;
  /** Back to reorder intent, with no wait running. */
  readonly reset: () => void;
};

/**
 * Switches a drag from reorder to merge intent. Hovering the centre of
 * one tile for MERGE_HOLD_MS makes it the merge target; moving to another
 * tile starts the wait again, and leaving the centre goes straight back
 * to reorder.
 */
export function useMergeIntent(): MergeIntent {
  const [target, setTarget] = useState<string | null>(null);
  const pending = useRef<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const stop = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
    pending.current = null;
  }, []);
  useEffect(() => stop, [stop]);

  const reset = useCallback(() => {
    stop();
    setTarget(null);
  }, [stop]);

  const hover = useCallback(
    (id: string | null) => {
      if (id === pending.current) return;
      reset();
      if (id === null) return;
      pending.current = id;
      timer.current = setTimeout(() => {
        timer.current = null;
        setTarget(id);
      }, MERGE_HOLD_MS);
    },
    [reset],
  );

  return { target, hover, stop, reset };
}

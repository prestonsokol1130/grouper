import { useCallback, useState } from "react";
import { clampPage } from "./paging";

/** The current page and the one it came from (still shown while sliding). */
export type Pager = {
  page: number;
  from: number;
  /** Shows page `index` (kept in range). */
  jump: (index: number) => void;
  /** Shows the page `step` pages on (negative goes back). */
  turn: (step: number) => void;
};

/**
 * The page shown out of `count`. Back to the first page on every show
 * (`showSeq` changes), and kept in range when pages go away.
 */
export function usePager(count: number, showSeq: number): Pager {
  const [state, setState] = useState({ page: 0, from: 0, seq: showSeq });
  // A new show starts on page 1 (adjusted while rendering, not in an effect).
  let current = state;
  if (state.seq !== showSeq) {
    current = { page: 0, from: 0, seq: showSeq };
    setState(current);
  }

  const jump = useCallback(
    (index: number) =>
      setState((previous) => {
        const page = clampPage(index, count);
        return page === previous.page ? previous : { ...previous, page, from: previous.page };
      }),
    [count],
  );
  const turn = useCallback(
    (step: number) =>
      setState((previous) => {
        const page = clampPage(clampPage(previous.page, count) + step, count);
        return page === previous.page ? previous : { ...previous, page, from: previous.page };
      }),
    [count],
  );

  return {
    page: clampPage(current.page, count),
    from: clampPage(current.from, count),
    jump,
    turn,
  };
}

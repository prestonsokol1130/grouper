/**
 * Keyboard movement in the flyout's paged grid. The page shape comes from
 * Rust (`flyout:show` carries the columns and rows of one page, see
 * `src-tauri/src/flyout/layout.rs`).
 */

/**
 * Where focus goes from tile `index` when `key` is pressed, in a grid of
 * `count` tiles and `columns` columns; `null` when the key does not move.
 * Left and right walk the reading order, up and down keep the column, and
 * down from a row above a short last row lands on the last tile.
 */
export function nextIndex(
  key: string,
  index: number,
  count: number,
  columns: number,
): number | null {
  if (count <= 0 || columns <= 0) return null;
  const last = count - 1;
  const row = (i: number) => Math.floor(i / columns);
  switch (key) {
    case "ArrowLeft":
      return Math.max(0, index - 1);
    case "ArrowRight":
      return Math.min(last, index + 1);
    case "ArrowUp":
      return index - columns >= 0 ? index - columns : index;
    case "ArrowDown":
      if (index + columns <= last) return index + columns;
      return row(index) < row(last) ? last : index;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}

/** A move in the paged grid: turn `step` pages, then focus tile `index`. */
export type PagedMove = { step: number; index: number };

/**
 * The move for `key` on tile `index` of page `page` (of `pages`), whose
 * `count` tiles sit in `columns` columns. Inside the page the arrows move
 * focus as {@link nextIndex}. Right from the last column (or the last
 * tile) goes to the next page, to the first column of the same row; Left
 * from the first column goes to the previous page, to the last column of
 * the same row. Page Up and Page Down turn a page, keeping the slot. The
 * caller clamps `index` to the tiles on the page it lands on. `null` when
 * nothing moves (an edge with no page beyond it, or another key).
 */
export function pagedMove(
  key: string,
  index: number,
  count: number,
  columns: number,
  page: number,
  pages: number,
): PagedMove | null {
  if (count <= 0 || columns <= 0) return null;
  const column = index % columns;
  const rowStart = index - column;
  const hasNext = page < pages - 1;
  const hasPrevious = page > 0;
  if (key === "PageDown") return hasNext ? { step: 1, index } : null;
  if (key === "PageUp") return hasPrevious ? { step: -1, index } : null;
  if (key === "ArrowRight" && (column === columns - 1 || index === count - 1)) {
    return hasNext ? { step: 1, index: rowStart } : null;
  }
  if (key === "ArrowLeft" && column === 0) {
    return hasPrevious ? { step: -1, index: rowStart + columns - 1 } : null;
  }
  const next = nextIndex(key, index, count, columns);
  return next === null || next === index ? null : { step: 0, index: next };
}

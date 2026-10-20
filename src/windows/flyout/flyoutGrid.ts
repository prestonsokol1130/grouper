/**
 * The flyout grid's shape and keyboard movement. The numbers mirror
 * `LAYOUT` in `src-tauri/src/flyout/layout.rs`, which sizes the window.
 */

/** DESIGN.md section 8: three apps across. */
const FLYOUT_MAX_COLUMNS = 3;

/** DESIGN.md section 7: the Medium flyout icon. */
export const FLYOUT_ICON_PX = 48;

/** Columns for `apps` apps: up to three; an empty group uses all three. */
export function flyoutColumns(apps: number): number {
  if (apps <= 0) return FLYOUT_MAX_COLUMNS;
  return Math.min(FLYOUT_MAX_COLUMNS, apps);
}

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

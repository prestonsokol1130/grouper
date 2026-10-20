import type { GridLayout } from "../../../lib/grid";

/**
 * The open folder's grid. DESIGN.md gives the folder no size, so it uses
 * the Launchpad's own cells with the flyout's 8px gap (section 8), and is
 * two columns and two rows smaller than the Launchpad grid (5 x 3 at the
 * default 7 x 5), never less than 3 x 2, so it always sits inside the
 * page with the Launchpad showing around it. A folder with more apps than
 * one page holds pages inside the panel.
 */

/** The flyout's gap between tiles (DESIGN.md section 8). */
const FOLDER_GAP = 8;

const MIN_COLUMNS = 3;
const MIN_ROWS = 2;

/** The grid for `count` apps inside a Launchpad laid out as `main`. */
export function folderGrid(count: number, main: GridLayout): GridLayout {
  const maxColumns = Math.max(MIN_COLUMNS, main.columns - 2);
  const maxRows = Math.max(MIN_ROWS, main.rows - 2);
  const columns = Math.max(MIN_COLUMNS, Math.min(maxColumns, count));
  const rows = Math.max(1, Math.min(maxRows, Math.ceil(count / columns)));
  return {
    columns,
    rows,
    perPage: columns * rows,
    cell: main.cell,
    gap: { column: FOLDER_GAP, row: FOLDER_GAP },
  };
}

/** `items` cut into pages of `perPage`; no apps is one empty page. */
export function folderPages<T>(items: readonly T[], perPage: number): T[][] {
  const size = Math.max(1, Math.floor(perPage));
  const pages: T[][] = [];
  for (let start = 0; start < items.length; start += size) {
    pages.push(items.slice(start, start + size));
  }
  return pages.length === 0 ? [[]] : pages;
}

/** The grid's size in px: its cells and the gaps between them. */
export function gridSize(grid: GridLayout): { width: number; height: number } {
  return {
    width: grid.columns * grid.cell.width + (grid.columns - 1) * grid.gap.column,
    height: grid.rows * grid.cell.height + (grid.rows - 1) * grid.gap.row,
  };
}

/**
 * The main grid slot under `point` in a grid drawn at `rect`: the cell
 * the pointer is over (or nearest to), counted in reading order.
 */
export function slotAt(
  point: { x: number; y: number },
  rect: { left: number; top: number },
  grid: GridLayout,
): number {
  const pitchX = grid.cell.width + grid.gap.column;
  const pitchY = grid.cell.height + grid.gap.row;
  const clamp = (value: number, max: number) => Math.max(0, Math.min(max, value));
  const column = clamp(
    Math.floor((point.x - rect.left + grid.gap.column / 2) / pitchX),
    grid.columns - 1,
  );
  const row = clamp(Math.floor((point.y - rect.top + grid.gap.row / 2) / pitchY), grid.rows - 1);
  return row * grid.columns + column;
}

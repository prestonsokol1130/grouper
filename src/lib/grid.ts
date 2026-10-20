/**
 * Launchpad grid sizing (DESIGN.md section 8). Pure. The grid keeps the
 * "Launchpad grid" setting (7 x 5 by default) wherever it fits: bigger
 * screens get more space between the tiles, not more tiles. Only a screen
 * too small to fit the setting at the minimum gap gets fewer columns or
 * rows.
 */

/** The window's size in CSS px. */
export type Viewport = { readonly width: number; readonly height: number };

/** The "Launchpad grid" setting and the "Icon size" setting in px. */
export type GridDensity = {
  readonly columns: number;
  readonly rows: number;
  readonly iconPx: number;
};

/** Where the tiles go: the grid that fits, its cell and the gaps, in px. */
export type GridLayout = {
  readonly columns: number;
  readonly rows: number;
  readonly perPage: number;
  /** One tile: the icon, its 8px inset and the Caption label. */
  readonly cell: { readonly width: number; readonly height: number };
  readonly gap: { readonly column: number; readonly row: number };
};

/** The default grid: 7 x 5 Medium (72px) icons. */
export const DEFAULT_DENSITY: GridDensity = { columns: 7, rows: 5, iconPx: 72 };

/** Space around the icon inside a tile (AppTile's inset). */
const TILE_INSET = 8;
/** The gap between icon and label, and the Caption line height. */
const LABEL_GAP = 4;
const LABEL_HEIGHT = 16;

/**
 * Space above and below the grid: the search pill (24px margin, 36px,
 * 24px margin) and the 48px page dots.
 */
const CHROME = { top: 24 + 36 + 24, bottom: 48 } as const;

/** Side padding: 8% of the width, at least 48px (as in S5-01). */
function sidePadding(width: number): number {
  return Math.max(48, Math.round(width * 0.08));
}

/** The smallest gap between tiles (Fluent 8px step). */
export const MIN_GAP = 8;

/**
 * The largest gaps, so an ultrawide or very tall screen centres a grid
 * with sensible spacing instead of scattering the tiles: two icons across
 * and one icon down.
 */
function maxGap(iconPx: number): { column: number; row: number } {
  return { column: iconPx * 2, row: iconPx };
}

/** One tile's size for `iconPx`. */
function cellSize(iconPx: number): { width: number; height: number } {
  return {
    width: iconPx + 2 * TILE_INSET,
    height: iconPx + 2 * TILE_INSET + LABEL_GAP + LABEL_HEIGHT,
  };
}

/** How many cells of `cell` px fit in `space` with at least MIN_GAP between them. */
function fit(space: number, cell: number, wanted: number): number {
  const fits = Math.floor((space + MIN_GAP) / (cell + MIN_GAP));
  return Math.max(1, Math.min(wanted, fits));
}

/** The gap that spreads `count` cells over `space`, between MIN_GAP and `max`. */
function spread(space: number, cell: number, count: number, max: number): number {
  if (count < 2) return MIN_GAP;
  const even = (space - count * cell) / (count - 1);
  return Math.floor(Math.min(max, Math.max(MIN_GAP, even)));
}

/** The grid for a window of `viewport` (see the file comment). */
export function computeGrid(
  viewport: Viewport,
  density: GridDensity = DEFAULT_DENSITY,
): GridLayout {
  const cell = cellSize(density.iconPx);
  const width = Math.max(0, viewport.width - 2 * sidePadding(viewport.width));
  const height = Math.max(0, viewport.height - CHROME.top - CHROME.bottom);
  const columns = fit(width, cell.width, Math.max(1, Math.floor(density.columns)));
  const rows = fit(height, cell.height, Math.max(1, Math.floor(density.rows)));
  const max = maxGap(density.iconPx);
  return {
    columns,
    rows,
    perPage: columns * rows,
    cell,
    gap: {
      column: spread(width, cell.width, columns, max.column),
      row: spread(height, cell.height, rows, max.row),
    },
  };
}

/**
 * The pages to show at `perPage` items a page. Stored pages are kept as
 * they are; a page with more items than fit is split, its overflow shown
 * on extra pages right after it, so the next stored page still starts a
 * page of its own. Empty pages are dropped. Display only: the stored
 * layout is never rewritten because the screen changed.
 */
export function reflowPages<T>(pages: readonly (readonly T[])[], perPage: number): T[][] {
  const size = Math.max(1, Math.floor(perPage));
  const shown: T[][] = [];
  for (const page of pages) {
    for (let start = 0; start < page.length; start += size) {
      shown.push(page.slice(start, start + size));
    }
  }
  return shown;
}

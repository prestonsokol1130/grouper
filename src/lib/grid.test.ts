import { describe, expect, it } from "vitest";
import { computeGrid, DEFAULT_DENSITY, MIN_GAP, reflowPages } from "./grid";

describe("computeGrid", () => {
  it("keeps 7 x 5 on common screens, with more space on bigger ones", () => {
    // 4K at 300% and 1080p at 100%.
    const small = computeGrid({ width: 1280, height: 720 });
    const large = computeGrid({ width: 1920, height: 1080 });
    for (const grid of [small, large]) {
      expect([grid.columns, grid.rows, grid.perPage]).toEqual([7, 5, 35]);
      expect(grid.cell).toEqual({ width: 88, height: 108 });
    }
    expect(small.gap).toEqual({ column: 76, row: 12 });
    expect(large.gap.column).toBeGreaterThan(small.gap.column);
    expect(large.gap.row).toBeGreaterThan(small.gap.row);
  });

  it("caps the gaps on ultrawide and very tall screens", () => {
    const ultrawide = computeGrid({ width: 5120, height: 1440 });
    expect([ultrawide.columns, ultrawide.rows]).toEqual([7, 5]);
    expect(ultrawide.gap).toEqual({ column: 144, row: 72 });
  });

  it("shrinks only when the setting no longer fits at the minimum gap", () => {
    // 5 rows need 5 * 108 + 4 * 8 = 572px of grid, plus 132px of chrome.
    expect(computeGrid({ width: 1280, height: 704 }).rows).toBe(5);
    expect(computeGrid({ width: 1280, height: 703 }).rows).toBe(4);
    const tiny = computeGrid({ width: 500, height: 400 });
    expect([tiny.columns, tiny.rows, tiny.perPage]).toEqual([4, 2, 8]);
    expect(tiny.gap.column).toBeGreaterThanOrEqual(MIN_GAP);
  });

  it("never goes below one tile, even in a window too small for one", () => {
    const none = computeGrid({ width: 0, height: 0 });
    expect([none.columns, none.rows, none.perPage]).toEqual([1, 1, 1]);
  });

  it("follows the grid and icon size settings", () => {
    const grid = computeGrid({ width: 1920, height: 1080 }, { columns: 5, rows: 4, iconPx: 96 });
    expect([grid.columns, grid.rows]).toEqual([5, 4]);
    expect(grid.cell).toEqual({ width: 112, height: 132 });
    expect(computeGrid({ width: 1920, height: 1080 }, DEFAULT_DENSITY).perPage).toBe(35);
  });
});

describe("reflowPages", () => {
  it("leaves pages that fit alone", () => {
    expect(reflowPages([[1, 2], [3]], 35)).toEqual([[1, 2], [3]]);
  });

  it("shows a stored page's overflow on extra pages right after it", () => {
    const stored = [[1, 2, 3, 4, 5], [6, 7], [8]];
    expect(reflowPages(stored, 2)).toEqual([[1, 2], [3, 4], [5], [6, 7], [8]]);
    // The stored layout is not touched.
    expect(stored).toEqual([[1, 2, 3, 4, 5], [6, 7], [8]]);
  });

  it("drops empty pages and treats a page size below 1 as 1", () => {
    expect(reflowPages([[], [1], []], 3)).toEqual([[1]]);
    expect(reflowPages([[1, 2]], 0)).toEqual([[1], [2]]);
    expect(reflowPages([], 35)).toEqual([]);
  });
});

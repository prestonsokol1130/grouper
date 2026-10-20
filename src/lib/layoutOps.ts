/**
 * Moving Launchpad items between pages. Pure: every function returns new
 * arrays and leaves its input alone. Used on the stored layout (logical
 * pages) and, for the drag preview only, on the pages a screen shows.
 */

/** A slot: page `page`, position `index` on that page. */
export type Position = { readonly page: number; readonly index: number };

/** Says whether an item takes up a slot (hidden apps do not). */
type Counts<T> = (item: T) => boolean;

const always = () => true;

/** How many of `items` take up a slot. */
function used<T>(items: readonly T[], counts: Counts<T>): number {
  return items.reduce((total, item) => total + (counts(item) ? 1 : 0), 0);
}

/**
 * Puts `item` at `to` (a page past the last one adds a page), then moves
 * whatever no longer fits onto the start of the next page, page after
 * page, so every page keeps at most `perPage` items that count. Items
 * that do not count ride along with their neighbours.
 */
export function insertWithOverflow<T>(
  pages: readonly (readonly T[])[],
  item: T,
  to: Position,
  perPage: number,
  counts: Counts<T> = always,
): T[][] {
  const size = Math.max(1, Math.floor(perPage));
  const result = pages.map((page) => [...page]);
  const target = Math.max(0, Math.min(to.page, result.length));
  if (target === result.length) result.push([]);
  const first = result[target] ?? [];
  first.splice(Math.max(0, Math.min(to.index, first.length)), 0, item);
  for (let index = target; index < result.length; index += 1) {
    const page = result[index] ?? [];
    const spill: T[] = [];
    while (used(page, counts) > size) {
      const last = page.pop();
      if (last === undefined) break;
      spill.unshift(last);
      // A hidden item at the end goes too, so the visible one is next.
      for (let tail = page.at(-1); tail !== undefined && !counts(tail); tail = page.at(-1)) {
        spill.unshift(tail);
        page.pop();
      }
    }
    if (spill.length === 0) break;
    if (index + 1 === result.length) result.push([]);
    result[index + 1] = [...spill, ...(result[index + 1] ?? [])];
  }
  return result;
}

/** `pages` without its empty pages at the end. */
function trimEnd<T>(pages: T[][]): T[][] {
  let end = pages.length;
  while (end > 0 && pages[end - 1]?.length === 0) end -= 1;
  return pages.slice(0, end);
}

/**
 * Moves the item at `from` so it ends up at `to`. `to` counts positions
 * after the item has been taken out, so a move within one page works like
 * moving an entry in a list; `to.page` one past the last page adds a page.
 * A page that ends up with more than `perPage` items that count pushes
 * its overflow onto the next page (see {@link insertWithOverflow}). A page
 * the move emptied is removed, and so is any empty page at the end. An
 * unknown `from` returns the pages unchanged.
 */
export function moveItem<T>(
  pages: readonly (readonly T[])[],
  from: Position,
  to: Position,
  perPage: number,
  counts: Counts<T> = always,
): T[][] {
  const source = pages[from.page];
  const item = source?.[from.index];
  if (source === undefined || item === undefined || from.index < 0) {
    return pages.map((page) => [...page]);
  }
  const without = pages.map((page, index) =>
    index === from.page ? page.filter((_, at) => at !== from.index) : [...page],
  );
  const moved = insertWithOverflow(without, item, to, perPage, counts);
  const emptied = moved[from.page]?.length === 0;
  const kept = emptied ? moved.filter((_, index) => index !== from.page) : moved;
  return trimEnd(kept);
}

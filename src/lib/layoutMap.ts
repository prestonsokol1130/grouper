/**
 * Turning a Launchpad drop on screen into a move of the stored layout.
 * Pure. The stored pages are logical pages and may hold hidden apps; the
 * screen shows only the visible items, and splits a stored page that has
 * more of them than its grid fits across several screens (`reflowPages`).
 */
import { insertWithOverflow, type Position } from "./layoutOps";

type Pages<T> = readonly (readonly T[])[];

/** The stored page and the first visible item each screen page shows. */
type Shown = { readonly page: number; readonly start: number };

/** Where every screen page comes from, matching `reflowPages`. */
function shownPages(visibleCounts: readonly number[], perPage: number): Shown[] {
  const size = Math.max(1, Math.floor(perPage));
  const shown: Shown[] = [];
  visibleCounts.forEach((count, page) => {
    for (let start = 0; start < count; start += size) shown.push({ page, start });
  });
  return shown;
}

/** Where the item with `key` is stored, if anywhere. */
export function findItem<T>(
  pages: Pages<T>,
  key: (item: T) => string,
  id: string,
): Position | null {
  for (const [page, items] of pages.entries()) {
    const index = items.findIndex((item) => key(item) === id);
    if (index >= 0) return { page, index };
  }
  return null;
}

/**
 * The slot an item dropped on screen page `page` lands in, before any
 * overflow: the first `perPage - 1` slots of a full page, else any slot up
 * to its end. `length` is how many other items the page shows.
 */
export function clampSlot(index: number, length: number, perPage: number): number {
  const last = length >= perPage ? Math.max(0, perPage - 1) : length;
  return Math.max(0, Math.min(index, last));
}

/** A move of the stored layout: take the item at `from`, put it at `to`. */
export type StoredMove = { readonly from: Position; readonly to: Position };

/**
 * Converts a drop on screen into stored positions for `moveItem`.
 *
 * `drop.page` is a screen page as shown when the drag started (one past
 * the last is the new page a drag may open); `drop.index` is where the
 * item goes among the other items that page shows. The target is the
 * stored slot just before the item it lands in front of, or just after
 * the page's last item when it goes at the end; dropped on the new page
 * it starts a new stored page. `to` counts positions with the item taken
 * out, as `moveItem` expects. Returns null when the item is not stored or
 * the drop would leave it where it is.
 */
export function displayToStored<T>(
  stored: Pages<T>,
  visible: (item: T) => boolean,
  perPage: number,
  key: (item: T) => string,
  activeId: string,
  drop: Position,
): StoredMove | null {
  const from = findItem(stored, key, activeId);
  if (from === null) return null;
  const without = stored.map((items, page) =>
    page === from.page ? items.filter((_, index) => index !== from.index) : items,
  );
  const screens = shownPages(
    stored.map((items) => items.filter(visible).length),
    perPage,
  );
  const screen = screens[drop.page];
  if (screen === undefined) {
    if (drop.page !== screens.length) return null;
    return { from, to: { page: without.length, index: 0 } };
  }
  const size = Math.max(1, Math.floor(perPage));
  const others = (stored[screen.page] ?? [])
    .filter(visible)
    .slice(screen.start, screen.start + size)
    .filter((item) => key(item) !== activeId);
  const page = without[screen.page] ?? [];
  const slot = clampSlot(drop.index, others.length, size);
  const before = others[slot];
  const after = others[slot - 1];
  let index: number;
  if (before !== undefined) index = page.indexOf(before);
  else if (after !== undefined) index = page.indexOf(after) + 1;
  else return null;
  const to = { page: screen.page, index };
  return to.page === from.page && to.index === from.index ? null : { from, to };
}

/**
 * What the screen shows while an item is dragged to `drop` (as in
 * {@link displayToStored}): `shown` with the item taken off its page and
 * put in its slot, a full page's overflow moved onto the next page. The
 * page it left stays, even when empty, so page numbers hold still.
 */
export function previewPages<T>(
  shown: Pages<T>,
  key: (item: T) => string,
  activeId: string,
  drop: Position,
  perPage: number,
): T[][] {
  const from = findItem(shown, key, activeId);
  const item = from === null ? undefined : shown[from.page]?.[from.index];
  if (from === null || item === undefined) return shown.map((items) => [...items]);
  const without = shown.map((items, page) =>
    page === from.page ? items.filter((_, index) => index !== from.index) : [...items],
  );
  const target = without[drop.page] ?? [];
  const slot = clampSlot(drop.index, target.length, perPage);
  return insertWithOverflow(without, item, { page: drop.page, index: slot }, perPage);
}

import { useEffect, useMemo, useRef, type KeyboardEvent } from "react";
import type { LaunchpadItem } from "../../bindings";
import { AppTile } from "../../components/AppTile";
import type { GridLayout } from "../../lib/grid";
import { folderPages, gridSize } from "../launchpad/folder/folderLayout";
import { PageDots } from "../launchpad/PageDots";
import { PagedGrid } from "../launchpad/PagedGrid";
import { usePager } from "../launchpad/usePager";
import { pagedMove } from "./flyoutGrid";

/** AppTile around its icon: an 8px inset each side, and inset, label gap,
 * Caption line and inset down (`CELL_EXTRA_*` in Rust's flyout layout). */
const CELL_EXTRA = { width: 16, height: 36 } as const;
/** DESIGN.md section 8: 8px between tiles. */
const GAP = 8;

type Props = {
  apps: readonly string[];
  /** One page's shape and the icon side, from `flyout:show`. */
  columns: number;
  rows: number;
  iconPx: number;
  /** Changes on every show: back to page 1 without sliding. */
  seq: number;
  reduced: boolean;
  showArrows: boolean;
  onLaunch: (appId: string) => void;
};

/** The tiles of the page on screen inside `area`, in grid order. */
export function currentTiles(area: HTMLElement | null): HTMLElement[] {
  if (area === null) return [];
  return [...area.querySelectorAll<HTMLElement>(".launchpad-page:not([inert]) .tile")];
}

/** The keys that turn pages when focus is not on a tile. */
const PAGE_KEYS: Readonly<Record<string, number>> = {
  ArrowLeft: -1,
  PageUp: -1,
  ArrowRight: 1,
  PageDown: 1,
};

/**
 * The flyout's apps in fixed pages of `columns` x `rows`, like an iPhone
 * folder (DESIGN.md section 8). Pages turn with a horizontal swipe or
 * drag, the mouse wheel (one notch, one page), the dots, the optional
 * ‹ › arrows and the keyboard: inside the grid the arrows move focus and
 * carry on to the next or previous page past its edge (see pagedMove);
 * with focus elsewhere Left/Right and Page Up/Down turn pages. The slide
 * is the Launchpad's (PagedGrid: transform only, 250ms).
 */
export function FlyoutPages(props: Props) {
  const { apps, columns, rows, iconPx, seq, reduced, showArrows, onLaunch } = props;
  const area = useRef<HTMLDivElement>(null);
  const focusNext = useRef<number | null>(null);
  const grid: GridLayout = useMemo(
    () => ({
      columns,
      rows,
      perPage: columns * rows,
      cell: { width: iconPx + CELL_EXTRA.width, height: iconPx + CELL_EXTRA.height },
      gap: { column: GAP, row: GAP },
    }),
    [columns, rows, iconPx],
  );
  const pages = useMemo(
    () =>
      folderPages(
        apps.map((appId): LaunchpadItem => ({ kind: "app", appId })),
        grid.perPage,
      ),
    [apps, grid.perPage],
  );
  const pager = usePager(pages.length, seq);
  const { page, turn } = pager;
  const size = gridSize(grid);

  // After a keyboard page turn, focus follows onto the new page.
  useEffect(() => {
    const target = focusNext.current;
    focusNext.current = null;
    if (target === null) return;
    const tiles = currentTiles(area.current);
    tiles[Math.min(target, tiles.length - 1)]?.focus({ preventScroll: true });
  }, [page]);

  // Focus outside the grid (or none): the arrows and page keys page.
  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      const step = PAGE_KEYS[event.key];
      if (step === undefined || event.defaultPrevented) return;
      // A tile's keys are onKeyDown's (which prevents those it uses).
      if (event.target instanceof Element && event.target.closest(".launchpad-page")) return;
      event.preventDefault();
      turn(step);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [turn]);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const tiles = currentTiles(area.current);
    const index = tiles.findIndex((tile) => tile === document.activeElement);
    if (index < 0) return;
    const move = pagedMove(event.key, index, tiles.length, columns, page, pages.length);
    if (move === null) return;
    event.preventDefault();
    if (move.step === 0) {
      tiles[move.index]?.focus({ preventScroll: true });
      return;
    }
    focusNext.current = move.index;
    turn(move.step);
  };

  return (
    <div ref={area} className="flyout-pages flex shrink-0 flex-col" onKeyDown={onKeyDown}>
      <div className="flex" style={{ width: size.width, height: size.height }}>
        <PagedGrid
          pages={pages}
          pager={pager}
          grid={grid}
          reduced={reduced}
          showSeq={seq}
          renderCell={(item, current) =>
            item.kind === "app" ? (
              <AppTile
                appId={item.appId}
                size={iconPx}
                onActivate={current ? onLaunch : undefined}
              />
            ) : null
          }
        />
      </div>
      {pages.length > 1 && (
        <PageDots
          count={pages.length}
          page={page}
          onSelect={pager.jump}
          arrows={showArrows}
          height="h-8"
        />
      )}
    </div>
  );
}

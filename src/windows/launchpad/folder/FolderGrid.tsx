import { useDraggable } from "@dnd-kit/core";
import { useEffect, useRef, type KeyboardEvent } from "react";
import type { LaunchpadItem } from "../../../bindings";
import { AppTile } from "../../../components/AppTile";
import type { GridLayout } from "../../../lib/grid";
import { useSetting } from "../../../store/settings";
import { nextIndex } from "../../flyout/flyoutGrid";
import { PageDots } from "../PageDots";
import { PagedGrid } from "../PagedGrid";
import type { Pager } from "../usePager";
import { gridSize } from "./folderLayout";

type Props = {
  pages: readonly (readonly LaunchpadItem[])[];
  grid: GridLayout;
  pager: Pager;
  reduced: boolean;
  iconPx: number;
  launch: (appId: string) => void;
};

/** One app of the page on screen: a click launches it, a drag lifts it. */
function FolderCell({
  appId,
  iconPx,
  launch,
}: {
  appId: string;
  iconPx: number;
  launch: Props["launch"];
}) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, isDragging } = useDraggable({
    id: appId,
  });
  return (
    <div
      ref={setNodeRef}
      className="flex items-center justify-center"
      data-sortable=""
      data-dragging={isDragging || undefined}
      onPointerDown={(event) => listeners?.onPointerDown?.(event)}
    >
      <AppTile
        appId={appId}
        size={iconPx}
        onActivate={launch}
        button={{
          ref: setActivatorNodeRef,
          "aria-roledescription": attributes["aria-roledescription"],
          "aria-describedby": attributes["aria-describedby"],
        }}
      />
    </div>
  );
}

/** Keys that turn the folder's pages. */
const PAGE_KEYS: Readonly<Record<string, number>> = { PageUp: -1, PageDown: 1 };

/**
 * The folder's apps, paged inside the panel with the Launchpad's paging
 * (wheel, swipe, dots). The arrow keys move between tiles; Left on the
 * first tile and Right on the last turn the page, as do Page Up and Page
 * Down, and focus follows onto the new page.
 */
export function FolderGrid({ pages, grid, pager, reduced, iconPx, launch }: Props) {
  const showArrows = useSetting("showPageArrows");
  const area = useRef<HTMLDivElement>(null);
  const focusNext = useRef<"first" | "last" | null>(null);
  const size = gridSize(grid);

  useEffect(() => {
    const which = focusNext.current;
    focusNext.current = null;
    if (which === null || area.current === null) return;
    const tiles = area.current.querySelectorAll<HTMLElement>(".launchpad-page:not([inert]) .tile");
    (which === "first" ? tiles.item(0) : tiles.item(tiles.length - 1))?.focus({
      preventScroll: true,
    });
  }, [pager.page]);

  const turn = (step: number) => {
    const target = pager.page + step;
    if (target < 0 || target >= pages.length) return false;
    focusNext.current = step > 0 ? "first" : "last";
    pager.turn(step);
    return true;
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = PAGE_KEYS[event.key];
    if (step !== undefined) {
      event.preventDefault();
      turn(step);
      return;
    }
    const tiles = [
      ...event.currentTarget.querySelectorAll<HTMLElement>(".launchpad-page:not([inert]) .tile"),
    ];
    const index = tiles.findIndex((tile) => tile === document.activeElement);
    if (index < 0) return;
    const last = tiles.length - 1;
    if (
      (event.key === "ArrowRight" && index === last) ||
      (event.key === "ArrowLeft" && index === 0)
    ) {
      event.preventDefault();
      turn(event.key === "ArrowRight" ? 1 : -1);
      return;
    }
    const next = nextIndex(event.key, index, tiles.length, grid.columns);
    if (next === null) return;
    event.preventDefault();
    tiles[next]?.focus();
  };

  return (
    <div ref={area} onKeyDown={onKeyDown}>
      <div className="mx-auto flex" style={{ width: size.width, height: size.height }}>
        <PagedGrid
          pages={pages}
          pager={pager}
          grid={grid}
          reduced={reduced}
          showSeq={0}
          renderCell={(item, current) =>
            item.kind !== "app" ? null : current ? (
              <FolderCell appId={item.appId} iconPx={iconPx} launch={launch} />
            ) : (
              <div className="flex items-center justify-center">
                <AppTile appId={item.appId} size={iconPx} />
              </div>
            )
          }
        />
      </div>
      {pages.length > 1 && (
        <PageDots
          count={pages.length}
          page={pager.page}
          onSelect={pager.jump}
          arrows={showArrows}
        />
      )}
    </div>
  );
}

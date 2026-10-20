import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  MeasuringStrategy,
  PointerSensor,
  useSensor,
  useSensors,
  type DropAnimation,
  type MeasuringConfiguration,
} from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { useCallback, useMemo, type ReactNode } from "react";
import { createPortal } from "react-dom";
import type { LaunchpadItem } from "../../bindings";
import type { TileButtonProps } from "../../components/activation";
import type { GridLayout } from "../../lib/grid";
import { displayToStored } from "../../lib/layoutMap";
import { moveItem, type Position } from "../../lib/layoutOps";
import { PAGE_SWIPE } from "../../lib/motion";
import { useAppStore } from "../../store/appStore";
import { itemKey, launchpadDensity, storedPerPage, visibleItem } from "./launchpadGrid";
import { PagedGrid } from "./PagedGrid";
import { SortableTile } from "./SortableTile";
import { measureSettled } from "./trackMeasure";
import type { Pager } from "./usePager";
import { useReorder, type Drag } from "./useReorder";

/** A press must move this far (px) before it picks a tile up; less is a click. */
const PICK_UP_PX = 8;

/** Tiles are measured where the page track is heading (see trackMeasure). */
const MEASURING: MeasuringConfiguration = {
  droppable: { strategy: MeasuringStrategy.Always, measure: measureSettled },
};

/** The lifted tile settles into its slot with the page-swipe timing. */
const DROP: DropAnimation = { duration: PAGE_SWIPE.durationMs, easing: PAGE_SWIPE.easing };

type Props = {
  /** The pages on screen; during a drag, with an empty page at the end. */
  pages: readonly (readonly LaunchpadItem[])[];
  drag: Drag | null;
  setDrag: (drag: Drag | null) => void;
  pager: Pager;
  grid: GridLayout;
  reduced: boolean;
  showSeq: number;
  /** A tile; `button` makes it a drag handle, `lifted` draws it picked up. */
  renderTile: (item: LaunchpadItem, button?: TileButtonProps, lifted?: boolean) => ReactNode;
};

/** Saves tile `id` dropped on screen slot `drop` (optimistic; a failure rolls back). */
function commitDrop(id: string, drop: Position, perPage: number): void {
  const store = useAppStore.getState();
  const state = store.state;
  if (state === null) return;
  const stored = state.launchpad?.pages ?? [];
  const visible = visibleItem(state);
  const move = displayToStored(stored, visible, perPage, itemKey, id, drop);
  if (move === null) return;
  const pages = moveItem(stored, move.from, move.to, storedPerPage(), visible);
  void store.setLaunchpadLayout({ ...state.launchpad, pages });
}

/**
 * The paged grid with drag-and-drop reordering. A press on a tile of the
 * page on screen that moves PICK_UP_PX picks it up; it follows the pointer,
 * lifted (DESIGN.md sections 10 and 11), while the other tiles slide aside
 * and its cell shows the drop slot. Resting at the left or right edge turns
 * the page (useEdgeFlip). The drop saves the new order at once.
 */
export function ReorderGrid({
  pages,
  drag,
  setDrag,
  pager,
  grid,
  reduced,
  showSeq,
  renderTile,
}: Props) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: PICK_UP_PX } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["Space"], cancel: ["Escape"], end: ["Space", "Enter"] },
    }),
  );
  const commit = useCallback(
    (id: string, drop: Position) => commitDrop(id, drop, grid.perPage),
    [grid.perPage],
  );
  const reorder = useReorder({ shown: pages, perPage: grid.perPage, drag, setDrag, pager, commit });
  const current = reorder.pages[pager.page];
  const ids = useMemo(() => (current ?? []).map(itemKey), [current]);
  const iconPx = launchpadDensity().iconPx;

  const renderCell = (item: LaunchpadItem, onScreen: boolean) =>
    onScreen ? (
      <SortableTile id={itemKey(item)} iconPx={iconPx} reduced={reduced}>
        {(button) => renderTile(item, button)}
      </SortableTile>
    ) : (
      <div className="flex items-center justify-center">{renderTile(item)}</div>
    );

  const lifted = reorder.activeItem;
  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      measuring={MEASURING}
      autoScroll={false}
      {...reorder.handlers}
    >
      <SortableContext items={ids}>
        <PagedGrid
          pages={reorder.pages}
          pager={pager}
          grid={grid}
          reduced={reduced}
          showSeq={showSeq}
          renderCell={renderCell}
        />
      </SortableContext>
      {createPortal(
        <DragOverlay
          dropAnimation={reduced ? null : DROP}
          className="launchpad-lifted text-primary"
        >
          {lifted === null ? null : (
            <div className="flex h-full items-center justify-center">
              {renderTile(lifted, undefined, true)}
            </div>
          )}
        </DragOverlay>,
        document.body,
      )}
    </DndContext>
  );
}

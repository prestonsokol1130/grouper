import { DndContext, DragOverlay, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { useEffect, useMemo, useRef, type MouseEvent } from "react";
import { createPortal } from "react-dom";
import type { LaunchpadItem } from "../../../bindings";
import { AppTile } from "../../../components/AppTile";
import type { GridLayout } from "../../../lib/grid";
import { closeTiming, type OpenCloseStyle } from "../../../lib/motion";
import { useAppStore } from "../../../store/appStore";
import { useGroup } from "../../../store/hooks";
import { groupApps } from "../../../store/selectors";
import { motionName, playClose } from "../../flyout/flyoutMotion";
import { usePager } from "../usePager";
import { FolderActions } from "./FolderActions";
import { folderGrid, folderPages, gridSize } from "./folderLayout";
import { FolderGrid } from "./FolderGrid";
import { FolderName } from "./FolderName";
import { useFolderDrag } from "./useFolderDrag";
import { useFolderMotion, type Point } from "./useFolderMotion";
import type { Pager } from "../usePager";

/** As on the Launchpad: a press must move this far (px) to lift a tile. */
const PICK_UP_PX = 8;

/** Fluent ContentDialog padding; wide enough for both actions on one row. */
const PADDING = 24;
const MIN_WIDTH = 400;

type Props = {
  groupId: string;
  /** The folder tile's centre, where the panel opens from. */
  origin: Point | null;
  mainGrid: GridLayout;
  mainPager: Pager;
  iconPx: number;
  style: OpenCloseStyle;
  reduced: boolean;
  /** Launches an app and hides the Launchpad (a failure keeps it, with a toast). */
  launch: (appId: string) => void;
  /** Hides the whole Launchpad. */
  hide: () => void;
  /** An app is being dragged out onto the Launchpad grid. */
  onDragOut: () => void;
  /** The panel has finished closing; unmount it. */
  onClosed: () => void;
};

/**
 * An open Launchpad folder: a Fluent card (8px corners, the surface
 * stroke, an acrylic-like blur and tint over the Launchpad) that opens
 * from its tile with the flyout's animation. The header is the name
 * (click to rename), then the apps, then Launch All and Pin. A click
 * outside the card, or Escape, closes the folder and leaves the
 * Launchpad open; the Launchpad's own Escape then closes it. Tab moves
 * through the name, the apps and the actions; the arrows move in the grid.
 */
export function FolderOverlay(props: Props) {
  const { groupId, mainGrid, mainPager, iconPx, style, reduced, launch, hide } = props;
  const group = useGroup(groupId);
  const state = useAppStore((store) => store.state);
  const { panel, close } = useFolderMotion(props.origin, style, reduced, props.onClosed);

  const apps = useMemo(() => (state ? groupApps(state, groupId) : []), [state, groupId]);
  const grid = folderGrid(apps.length, mainGrid);
  const pages = useMemo(() => {
    const items = apps.map((app): LaunchpadItem => ({ kind: "app", appId: app.id ?? "" }));
    return folderPages(items, grid.perPage);
  }, [apps, grid.perPage]);
  const pager = usePager(pages.length, 0);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: PICK_UP_PX } }),
  );
  const drag = useFolderDrag({
    groupId,
    panel,
    mainGrid,
    mainPager,
    onOut: () => {
      if (panel.current) void playClose(panel.current, closeTiming(style, reduced).durationMs);
      props.onDragOut();
    },
    onDone: props.onClosed,
  });

  // A folder that went away (dissolved, deleted) closes at once.
  const gone = state !== null && group === undefined;
  const closed = props.onClosed;
  useEffect(() => {
    if (gone && !drag.dragging) closed();
  }, [gone, drag.dragging, closed]);

  useEffect(() => panel.current?.focus(), [panel]);

  // Escape closes the folder first; the Launchpad's keys are paused meanwhile.
  const dragging = useRef(false);
  useEffect(() => {
    dragging.current = drag.dragging;
  });
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || dragging.current) return;
      event.preventDefault();
      close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  // Clicks never reach the Launchpad behind (its background click closes it).
  const onScrimClick = (event: MouseEvent<HTMLDivElement>) => {
    event.stopPropagation();
    if (event.target === event.currentTarget) close();
  };

  const width = Math.max(MIN_WIDTH, gridSize(grid).width + 2 * PADDING);
  return (
    <div className="folder-scrim" data-out={drag.out || undefined} onClick={onScrimClick}>
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-label={group?.name ?? ""}
        tabIndex={-1}
        className="flyout-panel folder-panel flex flex-col text-primary outline-none"
        style={{ width, padding: PADDING }}
        data-motion={motionName(style, reduced)}
        data-testid="folder-overlay"
      >
        <div className="mb-4 flex shrink-0 items-center">
          <FolderName groupId={groupId} name={group?.name ?? ""} />
        </div>
        <DndContext sensors={sensors} autoScroll={false} {...drag.handlers}>
          <FolderGrid
            pages={pages}
            grid={grid}
            pager={pager}
            reduced={reduced}
            iconPx={iconPx}
            launch={launch}
          />
          {createPortal(
            <DragOverlay
              dropAnimation={reduced || drag.out ? null : undefined}
              className="launchpad-lifted text-primary"
            >
              {drag.lifted === null ? null : (
                <div className="flex h-full items-center justify-center">
                  <AppTile appId={drag.lifted} size={iconPx} preview="hover" />
                </div>
              )}
            </DragOverlay>,
            document.body,
          )}
        </DndContext>
        <FolderActions
          groupId={groupId}
          name={group?.name ?? ""}
          hasShortcut={group?.hasShortcut === true}
          empty={apps.length === 0}
          hide={hide}
        />
      </div>
    </div>
  );
}

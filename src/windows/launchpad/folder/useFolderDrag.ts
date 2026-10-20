import type { DragEndEvent, DragMoveEvent, DragStartEvent } from "@dnd-kit/core";
import { useRef, useState, type RefObject } from "react";
import type { LaunchpadSlot } from "../../../bindings";
import type { GridLayout } from "../../../lib/grid";
import { screenToStoredSlot } from "../../../lib/layoutMap";
import { useAppStore } from "../../../store/appStore";
import { visibleItem } from "../launchpadGrid";
import { useEdgeFlip } from "../useEdgeFlip";
import type { Pager } from "../usePager";
import { slotAt } from "./folderLayout";
import type { Point } from "./useFolderMotion";

type Options = {
  groupId: string;
  /** The folder panel: a drag past its edge takes the app out. */
  panel: RefObject<HTMLElement | null>;
  /** The Launchpad's own grid and pager, where the app is dropped. */
  mainGrid: GridLayout;
  mainPager: Pager;
  /** The app left the panel: the folder closes, the Launchpad grid shows. */
  onOut: () => void;
  /** The drag that left the panel ended (dropped or cancelled). */
  onDone: () => void;
};

/** Where the pointer is during a pointer drag, or null (keyboard). */
function pointerOf(event: DragMoveEvent | DragEndEvent): Point | null {
  const start = event.activatorEvent;
  if (!("clientX" in start) || typeof start.clientX !== "number") return null;
  const y = "clientY" in start && typeof start.clientY === "number" ? start.clientY : 0;
  return { x: start.clientX + event.delta.x, y: y + event.delta.y };
}

const outside = (point: Point, rect: DOMRect) =>
  point.x < rect.left || point.x > rect.right || point.y < rect.top || point.y > rect.bottom;

/**
 * The stored slot under `point` on the Launchpad page on screen, mapped
 * with the S5-04 helpers (screen slot, then stored slot).
 */
function dropSlot(point: Point, grid: GridLayout, page: number): LaunchpadSlot | null {
  const state = useAppStore.getState().state;
  const shown = document.querySelector(
    ".launchpad-main .launchpad-page:not([inert]) .launchpad-grid",
  );
  if (state === null || shown === null) return null;
  const index = slotAt(point, shown.getBoundingClientRect(), grid);
  const stored = state.launchpad?.pages ?? [];
  return screenToStoredSlot(stored, visibleItem(state), grid.perPage, { page, index });
}

/**
 * Dragging an app out of the open folder (DESIGN.md: past the panel's
 * edge the folder closes and the app drops onto the Launchpad grid at the
 * pointer). Inside the panel a drop puts the tile back. Out of it, a rest
 * at the screen's left or right edge turns the Launchpad's page, and the
 * drop calls remove_app_from_folder with the stored slot under the
 * pointer; a folder left with one app is dissolved by the backend.
 */
export function useFolderDrag({ groupId, panel, mainGrid, mainPager, onOut, onDone }: Options) {
  const [lifted, setLifted] = useState<string | null>(null);
  const [out, setOut] = useState(false);
  const outRef = useRef(false);
  const track = useEdgeFlip((step) => mainPager.turn(step));

  const finish = () => {
    track(null);
    setLifted(null);
    if (!outRef.current) return;
    outRef.current = false;
    setOut(false);
    onDone();
  };

  const onDragStart = ({ active }: DragStartEvent) => setLifted(String(active.id));
  const onDragMove = (event: DragMoveEvent) => {
    const point = pointerOf(event);
    if (point === null) return;
    if (outRef.current) {
      track(point.x);
      return;
    }
    const rect = panel.current?.getBoundingClientRect();
    if (rect === undefined || !outside(point, rect)) return;
    outRef.current = true;
    setOut(true);
    onOut();
  };
  const onDragEnd = (event: DragEndEvent) => {
    const point = pointerOf(event);
    if (outRef.current && point !== null) {
      const slot = dropSlot(point, mainGrid, mainPager.page);
      const appId = String(event.active.id);
      if (slot !== null) void useAppStore.getState().removeAppFromFolder(groupId, appId, slot);
    }
    finish();
  };

  return {
    lifted,
    out,
    dragging: lifted !== null,
    handlers: { onDragStart, onDragMove, onDragEnd, onDragCancel: finish },
  };
}

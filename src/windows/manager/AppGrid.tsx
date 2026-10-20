import { useRef, type KeyboardEvent } from "react";
import type { AppEntry } from "../../bindings";
import { AppTile } from "../../components/AppTile";
import { IconButton } from "../../components/Button";
import { FluentIcon } from "./FluentIcon";
import { moveItem } from "./useDragReorder";
import { useGridReorder } from "./useGridReorder";

/** App icon size in the Manager grid: the Launchpad medium (DESIGN.md section 7). */
const TILE_ICON_PX = 72;

type Props = {
  groupName: string;
  apps: AppEntry[];
  onOpen: (appId: string) => void;
  /** The shown apps in their new order. */
  onReorder: (ids: string[]) => void;
  onRemove: (appId: string) => void;
};

/**
 * The selected group's apps as tiles, in group order. Drag a tile, or use
 * Alt+arrow keys on it, to reorder; each tile has a Remove button that
 * shows on hover or keyboard focus, and Delete on a tile removes it too.
 * Arrow keys move between tiles; a click or Enter opens the app.
 */
export function AppGrid({ groupName, apps, onOpen, onReorder, onRemove }: Props) {
  const list = useRef<HTMLUListElement>(null);
  const ids = apps.map((app) => app.id ?? "");
  const drag = useGridReorder(ids, onReorder);

  const focusTile = (id: string | undefined) => {
    if (id === undefined) return;
    requestAnimationFrame(() =>
      list.current?.querySelector<HTMLElement>(`[data-app-id="${id}"] .tile`)?.focus(),
    );
  };

  const remove = (id: string) => {
    const index = ids.indexOf(id);
    onRemove(id);
    focusTile(ids[index + 1] ?? ids[index - 1]);
  };

  const onTileKey = (event: KeyboardEvent<HTMLElement>, id: string) => {
    if (!(event.target instanceof HTMLElement) || !event.target.classList.contains("tile")) return;
    const index = ids.indexOf(id);
    if (event.key === "Escape") drag.cancel();
    if (event.key === "Delete") {
      event.preventDefault();
      remove(id);
      return;
    }
    const back = event.key === "ArrowLeft" || event.key === "ArrowUp";
    const forward = event.key === "ArrowRight" || event.key === "ArrowDown";
    if ((!back && !forward) || index < 0) return;
    event.preventDefault();
    const to = index + (back ? -1 : 1);
    if (to < 0 || to >= ids.length) return;
    if (event.altKey) {
      onReorder(moveItem(ids, index, to));
      focusTile(id);
    } else {
      focusTile(ids[to]);
    }
  };

  return (
    <ul
      ref={list}
      aria-label={`Apps in ${groupName}`}
      className="flex flex-wrap content-start gap-2"
      data-reordering={drag.draggingId !== null}
    >
      {apps.map((app) => {
        const id = app.id ?? "";
        const name = app.name ?? "";
        const shift = drag.shift(id);
        const moved = shift.x !== 0 || shift.y !== 0;
        return (
          <li
            key={id}
            data-app-id={id}
            data-dragging={drag.draggingId === id}
            className="app-cell relative touch-none select-none"
            style={moved ? { translate: `${shift.x}px ${shift.y}px` } : undefined}
            onKeyDown={(event) => onTileKey(event, id)}
            {...drag.cellProps(id)}
          >
            <AppTile appId={id} size={TILE_ICON_PX} onActivate={onOpen} />
            <span
              className="app-remove absolute top-0 right-0 rounded-control bg-layer"
              data-no-drag=""
            >
              <IconButton
                label={`Remove ${name}`}
                aria-keyshortcuts="Delete"
                onClick={() => remove(id)}
              >
                <FluentIcon glyph="remove" />
              </IconButton>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

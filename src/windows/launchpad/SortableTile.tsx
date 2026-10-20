import { useSortable } from "@dnd-kit/sortable";
import type { CSSProperties, KeyboardEvent, ReactNode } from "react";
import type { TileButtonProps } from "../../components/activation";
import { PAGE_SWIPE } from "../../lib/motion";

type Props = {
  id: string;
  /** Icon side in px, for the drop slot. */
  iconPx: number;
  reduced: boolean;
  /** Set while a dragged app would merge into this tile (merge intent). */
  merge?: "app" | "folder";
  /** The tile, given the props that make its button the drag handle. */
  children: (button: TileButtonProps) => ReactNode;
};

/**
 * Tiles slide out of the dragged tile's way with the page-swipe timing
 * (DESIGN.md section 10: 250ms ease-out); with reduced motion they jump.
 */
const MOVE_ASIDE = { duration: PAGE_SWIPE.durationMs, easing: PAGE_SWIPE.easing };

/**
 * One grid cell of the current Launchpad page that can be dragged to a new
 * place. A press that moves past the pointer sensor's distance picks the
 * tile up (a short click still opens it); Space on the focused tile picks
 * it up for the keyboard, the arrows move it, Space or Enter drops it and
 * Escape puts it back. Enter on a tile that is not being moved opens it.
 * While dragged, the cell shows the drop slot and the lifted tile follows
 * the pointer in the drag overlay. With `merge` set the tile shows it is
 * the drop target of a merge (see launchpad.css).
 */
export function SortableTile({ id, iconPx, reduced, merge, children }: Props) {
  const {
    active,
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, transition: reduced ? null : MOVE_ASIDE });

  // Space starts a keyboard drag before the tile's own keys see it; during
  // a drag Space and Enter only drop, they never open the tile.
  const onKeyDownCapture = (event: KeyboardEvent<HTMLDivElement>) => {
    if (active !== null) {
      if (event.key === " " || event.key === "Enter") event.preventDefault();
      return;
    }
    if (event.key === " " && !event.repeat) listeners?.onKeyDown?.(event);
  };

  const style: CSSProperties = {
    transform: transform ? `translate3d(${transform.x}px, ${transform.y}px, 0)` : undefined,
    transition: transition ?? undefined,
  };

  return (
    <div
      ref={setNodeRef}
      className="relative flex items-center justify-center"
      style={style}
      data-sortable=""
      data-dragging={isDragging || undefined}
      data-merge={merge}
      onPointerDown={(event) => listeners?.onPointerDown?.(event)}
      onKeyDownCapture={onKeyDownCapture}
    >
      {isDragging && (
        <span
          aria-hidden="true"
          className="drop-slot absolute"
          style={{ width: iconPx, height: iconPx }}
          data-testid="drop-slot"
        />
      )}
      {children({
        ref: setActivatorNodeRef,
        "aria-roledescription": attributes["aria-roledescription"],
        "aria-describedby": attributes["aria-describedby"],
      })}
    </div>
  );
}

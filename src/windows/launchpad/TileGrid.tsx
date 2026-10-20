import type { KeyboardEventHandler, ReactNode } from "react";
import type { GridLayout } from "../../lib/grid";

type Props = {
  grid: GridLayout;
  label: string;
  /** Rows to lay out; a flat result list grows as long as it needs. */
  rows?: number;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement>;
  children: ReactNode;
};

/**
 * Tiles in `grid.columns` fixed cells with the computed gaps, centred in
 * the space around them (bigger screens get more space, not more tiles).
 */
export function TileGrid({ grid, label, rows = grid.rows, onKeyDown, children }: Props) {
  return (
    <div
      role="group"
      aria-label={label}
      className="launchpad-grid grid content-center justify-center"
      style={{
        gridTemplateColumns: `repeat(${grid.columns}, ${grid.cell.width}px)`,
        gridTemplateRows: `repeat(${rows}, ${grid.cell.height}px)`,
        columnGap: grid.gap.column,
        rowGap: grid.gap.row,
      }}
      onKeyDown={onKeyDown}
    >
      {children}
    </div>
  );
}

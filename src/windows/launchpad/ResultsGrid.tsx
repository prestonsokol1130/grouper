import type { KeyboardEvent, ReactNode, RefObject } from "react";
import type { LaunchpadItem } from "../../bindings";
import type { GridLayout } from "../../lib/grid";
import { itemKey } from "./launchpadGrid";
import { TileGrid } from "./TileGrid";

type Props = {
  query: string;
  results: readonly LaunchpadItem[];
  grid: GridLayout;
  resultsRef: RefObject<HTMLDivElement | null>;
  /** Up from the first row: back to the search box. */
  onLeaveTop: () => void;
  renderTile: (item: LaunchpadItem) => ReactNode;
};

/** The empty-results line, in the section 13 voice. */
function noMatches(query: string): string {
  return `No apps match “${query.trim()}”.`;
}

/** Focuses result tile `index` inside `container`, if there is one. */
export function focusResult(container: HTMLElement | null, index: number): boolean {
  const tile = container?.querySelectorAll<HTMLElement>(".tile").item(index);
  if (!tile) return false;
  tile.focus();
  tile.scrollIntoView?.({ block: "nearest" });
  return true;
}

/** The arrow keys' moves through a grid `columns` wide. */
const MOVES: Readonly<Record<string, (columns: number) => number>> = {
  ArrowLeft: () => -1,
  ArrowRight: () => 1,
  ArrowUp: (columns) => -columns,
  ArrowDown: (columns) => columns,
};

/**
 * What a search shows instead of the pages: every match, best first, in
 * one flat grid with the page grid's cells, scrolling when it is taller
 * than the screen. The arrow keys move focus between results; Enter or
 * Space on one opens it. No matches shows a friendly line instead.
 */
export function ResultsGrid({ query, results, grid, resultsRef, onLeaveTop, renderTile }: Props) {
  if (results.length === 0) {
    return (
      <p
        className="flex min-h-0 flex-1 items-center justify-center px-4 text-center type-subtitle text-secondary"
        role="status"
      >
        {noMatches(query)}
      </p>
    );
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const move = MOVES[event.key];
    if (move === undefined || !(event.target instanceof HTMLElement)) return;
    const tiles = [...event.currentTarget.querySelectorAll(".tile")];
    const index = tiles.indexOf(event.target);
    if (index < 0) return;
    event.preventDefault();
    const next = index + move(grid.columns);
    if (next < 0 && event.key === "ArrowUp") onLeaveTop();
    else focusResult(event.currentTarget, Math.min(Math.max(0, next), tiles.length - 1));
  };

  return (
    <div ref={resultsRef} className="launchpad-results min-h-0 flex-1 overflow-y-auto py-2">
      <TileGrid
        grid={grid}
        label="Search results"
        rows={Math.ceil(results.length / grid.columns)}
        onKeyDown={onKeyDown}
      >
        {results.map((item) => (
          <div key={itemKey(item)} className="flex items-center justify-center">
            {renderTile(item)}
          </div>
        ))}
      </TileGrid>
    </div>
  );
}

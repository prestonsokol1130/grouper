import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import type { LaunchpadItem } from "../../bindings";
import type { TileButtonProps } from "../../components/activation";
import { AppTile } from "../../components/AppTile";
import { FolderTile } from "../../components/FolderTile";
import { reflowPages } from "../../lib/grid";
import { usePrefersReducedMotion } from "../../lib/motion";
import { rank } from "../../lib/search";
import { useAppStore } from "../../store/appStore";
import { motionName, motionVars, useOpenCloseStyle } from "../flyout/flyoutMotion";
import { FolderOverlay } from "./folder/FolderOverlay";
import { useOpenFolder } from "./folder/useOpenFolder";
import { searchIndex, useLaunchpadDensity, visiblePages } from "./launchpadGrid";
import { PageDots } from "./PageDots";
import { ReorderGrid } from "./ReorderGrid";
import { focusResult, ResultsGrid } from "./ResultsGrid";
import { SearchBar } from "./SearchBar";
import { useLaunchpadClose } from "./useLaunchpadClose";
import { useLaunchpadKeys } from "./useLaunchpadKeys";
import { useLaunchpadShow } from "./useLaunchpadShow";
import { usePager } from "./usePager";
import { useDragState } from "./useReorder";
import { useViewportGrid } from "./useViewportGrid";

/** Shown while there are no apps at all (the first scan may still be running). */
const EMPTY_MESSAGE = "No apps yet";

/**
 * The fullscreen Launchpad. Mounted once and kept forever: each
 * `launchpad:show` replays the open animation from the origin Rust sends,
 * focuses the search pill and goes back to page 1. Without a search it
 * shows the layout's pages in a grid sized to the screen, turned by the
 * wheel, arrow and page keys, a drag or the page dots. Typing searches:
 * the pages give way to a flat list of results. Clicking an app launches
 * it and closes the Launchpad; clicking a folder opens it over the grid
 * (FolderOverlay). Clicking anywhere that is not a control, or Escape
 * with an empty search, closes it; with a folder open both close the
 * folder first. The window itself is shown
 * and hidden by Rust; only the content animates.
 */
export function LaunchpadRoot() {
  const { show, seq } = useLaunchpadShow();
  const style = useOpenCloseStyle();
  const reduced = usePrefersReducedMotion();
  const state = useAppStore((store) => store.state);
  const grid = useViewportGrid();
  const iconPx = useLaunchpadDensity().iconPx;
  const [query, setQuery] = useState("");
  const panel = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const results = useRef<HTMLDivElement>(null);

  const [drag, setDrag] = useDragState();
  const pages = useMemo(() => {
    const shown = reflowPages(visiblePages(state), grid.perPage);
    // A drag may open a new page after the last one.
    return drag === null ? shown : [...shown, []];
  }, [state, grid, drag]);
  const searching = query.trim() !== "";
  const matches = useMemo(
    () => (searching ? rank(query, searchIndex(state)) : []),
    [searching, query, state],
  );
  const pager = usePager(pages.length, seq);
  const folders = useOpenFolder();
  const { folder, reset: resetFolder } = folders;
  const { close, launch } = useLaunchpadClose({
    panel,
    seq,
    style,
    reduced,
    onHidden: () => {
      setQuery("");
      resetFolder();
    },
  });
  // Every show starts on the grid, with no folder open.
  useEffect(() => resetFolder(), [seq, resetFolder]);
  useLaunchpadKeys({
    query,
    setQuery,
    inputRef: input,
    close,
    turn: pager.turn,
    // A drag owns the keys; an open folder handles Escape and its own keys.
    paused: drag !== null || folder !== null,
  });

  const openFolder = folders.open;
  const open = useCallback(
    (item: LaunchpadItem) => {
      if (item.kind === "app") void launch(item.appId);
      else openFolder(item.groupId);
    },
    [launch, openFolder],
  );

  // Stable, so a memoised tile whose item did not change skips re-rendering.
  const launchApp = useCallback((appId: string) => void launch(appId), [launch]);
  const renderTile = useCallback(
    (item: LaunchpadItem, button?: TileButtonProps, lifted = false) => {
      const preview = lifted ? "hover" : undefined;
      const size = iconPx;
      return item.kind === "app" ? (
        <AppTile
          appId={item.appId}
          size={size}
          onActivate={launchApp}
          button={button}
          preview={preview}
        />
      ) : (
        <FolderTile
          groupId={item.groupId}
          size={size}
          onOpen={openFolder}
          button={button}
          preview={preview}
        />
      );
    },
    [launchApp, openFolder, iconPx],
  );

  // Anything that is not a control is background, the gaps in the grid too.
  const onBackgroundClick = (event: MouseEvent<HTMLDivElement>) => {
    if (event.target instanceof Element && event.target.closest("button, input")) return;
    close();
  };

  const firstMatch = matches[0];
  return (
    <div
      ref={panel}
      className="flyout-panel launchpad-panel flex h-screen flex-col text-primary"
      data-motion={motionName(style, reduced)}
      data-edge={show?.origin.edge ?? "bottom"}
      style={motionVars(style, reduced, show?.origin)}
      onClick={onBackgroundClick}
      data-testid="launchpad"
    >
      <div className="launchpad-main flex min-h-0 flex-1 flex-col" inert={folder !== null}>
        <div className="mt-6 mb-6 flex h-9 shrink-0 justify-center">
          <SearchBar
            value={query}
            onChange={setQuery}
            showSeq={seq}
            inputRef={input}
            onSubmit={() => firstMatch && open(firstMatch)}
            onFocusResults={() => focusResult(results.current, 0)}
          />
        </div>
        {searching ? (
          <ResultsGrid
            query={query}
            results={matches}
            grid={grid}
            resultsRef={results}
            onLeaveTop={() => input.current?.focus()}
            renderTile={renderTile}
          />
        ) : pages.length === 0 ? (
          <p className="flex flex-1 items-center justify-center type-title-large text-secondary">
            {EMPTY_MESSAGE}
          </p>
        ) : (
          <ReorderGrid
            pages={pages}
            drag={drag}
            setDrag={setDrag}
            pager={pager}
            grid={grid}
            reduced={reduced}
            showSeq={seq}
            renderTile={renderTile}
          />
        )}
        {searching ? (
          <div aria-hidden="true" className="h-12 shrink-0" />
        ) : (
          <PageDots count={pages.length} page={pager.page} onSelect={pager.jump} />
        )}
      </div>
      {folder !== null && (
        <FolderOverlay
          key={folder.groupId}
          groupId={folder.groupId}
          origin={folder.origin}
          mainGrid={grid}
          mainPager={pager}
          iconPx={iconPx}
          style={style}
          reduced={reduced}
          launch={(appId) => void launch(appId)}
          hide={close}
          onDragOut={() => setQuery("")}
          onClosed={folders.closed}
        />
      )}
    </div>
  );
}

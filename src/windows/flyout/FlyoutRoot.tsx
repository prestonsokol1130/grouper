import { useCallback, useEffect, useLayoutEffect, useRef, type KeyboardEvent } from "react";
import { AppTile } from "../../components/AppTile";
import { api } from "../../lib/api";
import { onEvent } from "../../lib/events";
import { openCloseTiming, usePrefersReducedMotion } from "../../lib/motion";
import { useAppStore } from "../../store/appStore";
import { useGroup } from "../../store/hooks";
import { appsById } from "../../store/selectors";
import { showToast } from "../../store/toastStore";
import { FLYOUT_ICON_PX, flyoutColumns, nextIndex } from "./flyoutGrid";
import {
  motionName,
  motionVars,
  playClose,
  playOpen,
  resetHidden,
  useOpenCloseStyle,
} from "./flyoutMotion";
import { useFlyoutShow } from "./useFlyoutShow";

/** DESIGN.md section 13. */
const EMPTY_MESSAGE = "No apps in this group yet. Drag apps here or click Add apps.";

/** Launches `appId`; on failure says so in the flyout, which stays open. */
async function launch(appId: string): Promise<void> {
  const result = await api.launchFromFlyout(appId);
  if (result.ok) return;
  console.error(`flyout: launch ${appId}: ${result.error}`);
  const state = useAppStore.getState().state;
  const name = state ? appsById(state).get(appId)?.name : undefined;
  showToast({
    kind: "error",
    title: name ? `Couldn't open ${name}.` : "Couldn't open the app.",
    message: "It may have been moved or uninstalled.",
  });
}

/** The tiles in `panel`, in grid order. */
function tilesIn(panel: HTMLElement | null): HTMLElement[] {
  return panel ? [...panel.querySelectorAll<HTMLElement>(".tile")] : [];
}

/**
 * The taskbar flyout. Mounted once and kept forever: each `flyout:show`
 * swaps in the group's data and replays the open animation. Shows the
 * group name and its apps three across; arrows move between tiles, Enter
 * or a click launches, Escape plays the close animation and hides.
 */
export function FlyoutRoot() {
  const { show, seq } = useFlyoutShow();
  const style = useOpenCloseStyle();
  const reduced = usePrefersReducedMotion();
  const group = useGroup(show?.groupId ?? "");
  const panel = useRef<HTMLDivElement>(null);
  const seqRef = useRef(seq);
  const closing = useRef(false);
  const apps = show?.apps ?? [];
  const columns = flyoutColumns(apps.length);

  useLayoutEffect(() => {
    seqRef.current = seq;
    closing.current = false;
    const element = panel.current;
    if (seq === 0 || element === null) return;
    playOpen(element);
    tilesIn(element)[0]?.focus();
  }, [seq]);

  // Rust hides the window on blur (or a launch) without waiting for an
  // animation; start the next show from nothing rather than this frame.
  useEffect(
    () =>
      onEvent("flyout:hide", () => {
        if (panel.current) resetHidden(panel.current);
      }),
    [],
  );

  const close = useCallback(() => {
    const element = panel.current;
    if (closing.current) return;
    closing.current = true;
    const started = seqRef.current;
    const timing = openCloseTiming(style, reduced);
    const played = element ? playClose(element, timing.durationMs) : Promise.resolve();
    void played.then(async () => {
      // A show that arrived during the animation wins.
      if (seqRef.current !== started) return;
      const result = await api.hideFlyout();
      if (!result.ok) console.error(`flyout: hide: ${result.error}`);
    });
  }, [style, reduced]);

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  const onGridKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const tiles = tilesIn(panel.current);
    const index = tiles.findIndex((tile) => tile === document.activeElement);
    if (index < 0) return;
    const next = nextIndex(event.key, index, tiles.length, columns);
    if (next === null) return;
    event.preventDefault();
    tiles[next]?.focus();
  };

  // The outer edge (DESIGN.md section 3) is an inset 1px ring so it takes
  // no space from the 12px padding the Rust size formula counts on.
  return (
    <div
      ref={panel}
      className="flyout-panel flex h-screen flex-col overflow-x-hidden overflow-y-auto rounded-card p-3 text-primary ring-1 ring-stroke-surface ring-inset"
      data-motion={motionName(style, reduced)}
      data-edge={show?.origin.edge ?? "bottom"}
      style={motionVars(style, reduced, show?.origin)}
    >
      {show !== null && (
        <>
          <h1 className="mb-2 shrink-0 truncate type-subtitle" title={group?.name}>
            {group?.name ?? ""}
          </h1>
          {apps.length === 0 ? (
            <p className="type-body text-secondary">{EMPTY_MESSAGE}</p>
          ) : (
            <div
              role="group"
              aria-label={group?.name}
              className="grid gap-2"
              style={{ gridTemplateColumns: `repeat(${columns}, max-content)` }}
              onKeyDown={onGridKey}
            >
              {apps.map((appId) => (
                <AppTile
                  key={appId}
                  appId={appId}
                  size={FLYOUT_ICON_PX}
                  onActivate={(id) => void launch(id)}
                />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

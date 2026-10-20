import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, type MouseEvent } from "react";
import { AppTile } from "../../components/AppTile";
import { FolderTile } from "../../components/FolderTile";
import { api } from "../../lib/api";
import { onEvent } from "../../lib/events";
import { openCloseTiming, usePrefersReducedMotion } from "../../lib/motion";
import { useAppStore } from "../../store/appStore";
import { appsById } from "../../store/selectors";
import { showToast } from "../../store/toastStore";
import {
  motionName,
  motionVars,
  playClose,
  playOpen,
  resetHidden,
  useOpenCloseStyle,
} from "../flyout/flyoutMotion";
import { GRID, itemKey, LAUNCHPAD_ICON_PX, pageItems } from "./launchpadGrid";
import { useLaunchpadShow } from "./useLaunchpadShow";

/** Shown while there is nothing on the first page (the first scan may still be running). */
const EMPTY_MESSAGE = "No apps yet";

/** Says a launch failed; the raw error goes to the log. */
function launchFailed(appId: string, error: string): void {
  console.error(`launchpad: launch ${appId}: ${error}`);
  const state = useAppStore.getState().state;
  const name = state ? appsById(state).get(appId)?.name : undefined;
  showToast({
    kind: "error",
    title: name ? `Couldn't open ${name}.` : "Couldn't open the app.",
    message: "It may have been moved or uninstalled.",
  });
}

/**
 * The fullscreen Launchpad. Mounted once and kept forever: each
 * `launchpad:show` replays the open animation from the origin Rust sends.
 * Shows page 1 of the layout as a 7 x 5 grid of app and folder tiles.
 * Clicking an app launches it and closes the Launchpad; clicking anywhere
 * that is not a tile, or Escape, closes it. The window itself is shown and
 * hidden by Rust; only the content animates.
 */
export function LaunchpadRoot() {
  const { show, seq } = useLaunchpadShow();
  const style = useOpenCloseStyle();
  const reduced = usePrefersReducedMotion();
  const state = useAppStore((store) => store.state);
  const items = useMemo(() => pageItems(state, 0), [state]);
  const panel = useRef<HTMLDivElement>(null);
  const seqRef = useRef(seq);
  const closing = useRef(false);

  useLayoutEffect(() => {
    seqRef.current = seq;
    closing.current = false;
    if (seq !== 0 && panel.current !== null) playOpen(panel.current);
  }, [seq]);

  // Rust hides the window on blur without waiting for an animation; the
  // next show then starts from nothing rather than this frame.
  useEffect(
    () =>
      onEvent("launchpad:hide", () => {
        if (panel.current) resetHidden(panel.current);
      }),
    [],
  );

  const close = useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    const started = seqRef.current;
    const element = panel.current;
    const timing = openCloseTiming(style, reduced);
    const played = element ? playClose(element, timing.durationMs) : Promise.resolve();
    void played.then(async () => {
      // A show that arrived during the animation wins.
      if (seqRef.current !== started) return;
      const result = await api.hideLaunchpad();
      if (result.ok) return;
      console.error(`launchpad: hide: ${result.error}`);
      // Still showing: open it back up so closing works again.
      closing.current = false;
      if (panel.current && seqRef.current === started) playOpen(panel.current);
    });
  }, [style, reduced]);

  const launch = useCallback(
    async (appId: string) => {
      const result = await api.launchApp(appId);
      if (result.ok) close();
      else launchFailed(appId, result.error);
    },
    [close],
  );

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  // Anything that is not a tile is background, the gaps in the grid too.
  const onBackgroundClick = (event: MouseEvent<HTMLDivElement>) => {
    if (event.target instanceof Element && event.target.closest(".tile")) return;
    close();
  };

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
      {/* The search pill (S5-02) goes here: 36px tall, 24px from the top. */}
      <div aria-hidden="true" className="mt-6 mb-6 h-9 shrink-0" />
      {items.length === 0 ? (
        <p className="flex flex-1 items-center justify-center type-title-large text-secondary">
          {EMPTY_MESSAGE}
        </p>
      ) : (
        <div
          role="group"
          aria-label="Apps"
          className="launchpad-grid grid min-h-0 flex-1"
          style={{
            gridTemplateColumns: `repeat(${GRID.columns}, minmax(0, 1fr))`,
            gridTemplateRows: `repeat(${GRID.rows}, minmax(0, 1fr))`,
          }}
        >
          {items.map((item) => (
            <div key={itemKey(item)} className="flex items-center justify-center">
              {item.kind === "app" ? (
                <AppTile
                  appId={item.appId}
                  size={LAUNCHPAD_ICON_PX}
                  onActivate={(id) => void launch(id)}
                />
              ) : (
                <FolderTile groupId={item.groupId} size={LAUNCHPAD_ICON_PX} />
              )}
            </div>
          ))}
        </div>
      )}
      {/* The page dots (S5-02) go here. */}
      <div aria-hidden="true" className="h-12 shrink-0" />
    </div>
  );
}

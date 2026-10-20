import { useCallback, useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import { api } from "../../lib/api";
import { onEvent } from "../../lib/events";
import { openCloseTiming, type OpenCloseStyle } from "../../lib/motion";
import { useAppStore } from "../../store/appStore";
import { appsById } from "../../store/selectors";
import { showToast } from "../../store/toastStore";
import { playClose, playOpen, resetHidden } from "../flyout/flyoutMotion";
import { setLaunchpadVisible } from "./pendingToast";

type Options = {
  panel: RefObject<HTMLDivElement | null>;
  seq: number;
  style: OpenCloseStyle;
  reduced: boolean;
  /** Runs when Rust hides the window (blur, toggle, or after a close). */
  onHidden: () => void;
};

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
 * The Launchpad's open and close: each show (`seq`) replays the open
 * animation; `close` plays the close one and then asks Rust to hide the
 * window; `launch` opens an app and closes on success.
 */
export function useLaunchpadClose({ panel, seq, style, reduced, onHidden }: Options) {
  const seqRef = useRef(seq);
  const closing = useRef(false);
  const hidden = useRef(onHidden);
  useEffect(() => {
    hidden.current = onHidden;
  });

  useLayoutEffect(() => {
    seqRef.current = seq;
    closing.current = false;
    if (seq !== 0 && panel.current !== null) playOpen(panel.current);
  }, [seq, panel]);

  // Rust hides the window on blur without waiting for an animation; the
  // next show then starts from nothing rather than this frame.
  useEffect(
    () =>
      onEvent("launchpad:hide", () => {
        setLaunchpadVisible(false);
        if (panel.current) resetHidden(panel.current);
        hidden.current();
      }),
    [panel],
  );

  const close = useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    // Hiding: a toast from now on waits for the next open.
    setLaunchpadVisible(false);
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
      setLaunchpadVisible(true);
      if (panel.current && seqRef.current === started) playOpen(panel.current);
    });
  }, [style, reduced, panel]);

  const launch = useCallback(
    async (appId: string) => {
      const result = await api.launchApp(appId);
      if (result.ok) close();
      else launchFailed(appId, result.error);
    },
    [close],
  );

  return { close, launch };
}

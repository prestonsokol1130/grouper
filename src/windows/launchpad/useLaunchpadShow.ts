import { useEffect, useState } from "react";
import { flushSync } from "react-dom";
import { onEvent, type LaunchpadShow } from "../../lib/events";
import { logInfo } from "../../logging";
import { showToast, useToastStore } from "../../store/toastStore";
import { setLaunchpadVisible, takePending } from "./pendingToast";

/** The last show and a counter that changes on every show. */
export type ShownLaunchpad = { show: LaunchpadShow | null; seq: number };

/**
 * Logs the time from receiving `launchpad:show` to the next animation
 * frame (the first paint after the commit) as `launchpad_paint_ms`, next
 * to Rust's `launchpad_show_us` in grouper.log.
 */
function measurePaint(receivedAt: number): void {
  requestAnimationFrame(() => {
    const ms = (performance.now() - receivedAt).toFixed(1);
    logInfo("launchpad", `launchpad_paint_ms=${ms}`);
  });
}

/**
 * Follows `launchpad:show`. Each show is committed synchronously, so the
 * first frame the window paints already starts the open animation, and
 * bumps `seq` so the caller replays it. Toasts from an earlier show are
 * cleared.
 */
export function useLaunchpadShow(): ShownLaunchpad {
  const [shown, setShown] = useState<ShownLaunchpad>({ show: null, seq: 0 });

  useEffect(
    () =>
      onEvent("launchpad:show", (payload) => {
        const at = performance.now();
        flushSync(() => {
          setLaunchpadVisible(true);
          useToastStore.setState({ toasts: [] });
          // A Launch All failure that came while hidden shows now.
          const kept = takePending();
          if (kept !== null) showToast(kept);
          setShown((previous) => ({ show: payload, seq: previous.seq + 1 }));
        });
        measurePaint(at);
      }),
    [],
  );

  return shown;
}

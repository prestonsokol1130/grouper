import { useEffect, useState } from "react";
import { flushSync } from "react-dom";
import type { FlyoutShow } from "../../bindings";
import { api } from "../../lib/api";
import { onEvent } from "../../lib/events";
import { logInfo } from "../../logging";
import { useToastStore } from "../../store/toastStore";

/** The group being shown and a counter that changes on every show. */
export type ShownFlyout = { show: FlyoutShow | null; seq: number };

/**
 * Logs the time from receiving `flyout:show` to the next animation frame
 * (the first paint after the swap) as `flyout_paint_ms`, next to Rust's
 * `flyout_show_us` in grouper.log.
 */
function measurePaint(receivedAt: number, groupId: string): void {
  requestAnimationFrame(() => {
    const ms = (performance.now() - receivedAt).toFixed(1);
    logInfo("flyout", `flyout_paint_ms=${ms} group=${groupId}`);
  });
}

/**
 * Follows `flyout:show`. Each show swaps the data synchronously (the store
 * already holds every app and group, so nothing is fetched) and bumps
 * `seq` so the caller replays the open animation. Toasts from an earlier
 * show are cleared. On mount it also asks for the last payload, in case
 * the page loaded after the first show.
 */
export function useFlyoutShow(): ShownFlyout {
  const [shown, setShown] = useState<ShownFlyout>({ show: null, seq: 0 });

  useEffect(() => {
    let received = false;
    const stop = onEvent("flyout:show", (payload) => {
      const at = performance.now();
      received = true;
      // Committed before the handler returns, so the first frame the
      // shown window paints already has this group.
      flushSync(() => {
        useToastStore.setState({ toasts: [] });
        setShown((previous) => ({ show: payload, seq: previous.seq + 1 }));
      });
      measurePaint(at, payload.groupId);
    });
    void api.getFlyout().then((result) => {
      if (!result.ok) console.error(`flyout: get_flyout: ${result.error}`);
      else if (!received && result.value) {
        const current = result.value;
        setShown((previous) => ({ show: current, seq: previous.seq + 1 }));
      }
    });
    return stop;
  }, []);

  return shown;
}

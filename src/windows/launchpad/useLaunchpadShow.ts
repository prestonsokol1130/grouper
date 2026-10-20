import { useEffect, useState } from "react";
import { flushSync } from "react-dom";
import { onEvent, type LaunchpadShow } from "../../lib/events";
import { useToastStore } from "../../store/toastStore";

/** The last show and a counter that changes on every show. */
export type ShownLaunchpad = { show: LaunchpadShow | null; seq: number };

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
        flushSync(() => {
          useToastStore.setState({ toasts: [] });
          setShown((previous) => ({ show: payload, seq: previous.seq + 1 }));
        });
      }),
    [],
  );

  return shown;
}

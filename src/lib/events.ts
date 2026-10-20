import { listen } from "@tauri-apps/api/event";
import type { AccentColors, AppState, FlyoutOrigin, FlyoutShow } from "../bindings";

/** The `launchpad:show` payload: where the open animation starts. */
export type LaunchpadShow = { origin: FlyoutOrigin };

/** The `hotkey:failed` payload: the shortcut and the raw error (for the log). */
export type HotkeyFailed = { hotkey: string; error: string };

/**
 * Backend events and their payloads. The names match the Rust constants
 * `STATE_CHANGED_EVENT`, `ICONS_UPDATED_EVENT`, `ACCENT_CHANGED_EVENT`
 * `FLYOUT_SHOW_EVENT`, `FLYOUT_HIDE_EVENT`, `LAUNCHPAD_SHOW_EVENT`,
 * `LAUNCHPAD_HIDE_EVENT` and `HOTKEY_FAILED_EVENT`.
 */
type Events = {
  "state:changed": AppState;
  "icons:updated": string[];
  "accent:changed": AccentColors | null;
  "flyout:show": FlyoutShow;
  "flyout:hide": null;
  "launchpad:show": LaunchpadShow;
  "launchpad:hide": null;
  "hotkey:failed": HotkeyFailed;
};

/**
 * Listens to backend event `name`. Returns a function that stops
 * listening, usable at once even though registration is asynchronous; a
 * failed registration is logged.
 */
export function onEvent<K extends keyof Events>(
  name: K,
  handler: (payload: Events[K]) => void,
): () => void {
  let stopped = false;
  let stop: (() => void) | null = null;
  listen<Events[K]>(name, (event) => handler(event.payload))
    .then((unlisten) => {
      if (stopped) unlisten();
      else stop = unlisten;
    })
    .catch((error: unknown) => console.error(`listen ${name}:`, error));
  return () => {
    stopped = true;
    stop?.();
  };
}

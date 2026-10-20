import { listen } from "@tauri-apps/api/event";
import type { AccentColors, AppState, FlyoutShow } from "../bindings";

/**
 * Backend events and their payloads. The names match the Rust constants
 * `STATE_CHANGED_EVENT`, `ICONS_UPDATED_EVENT`, `ACCENT_CHANGED_EVENT`
 * and `FLYOUT_SHOW_EVENT`.
 */
type Events = {
  "state:changed": AppState;
  "icons:updated": string[];
  "accent:changed": AccentColors | null;
  "flyout:show": FlyoutShow;
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

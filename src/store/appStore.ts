import { create } from "zustand";
import type { AppState, LaunchpadLayout } from "../bindings";
import { api, type Result } from "../lib/api";
import { onEvent } from "../lib/events";
import { withGroupApps, withGroupRenamed, withLaunchpadLayout } from "./reducers";
import { showToast } from "./toastStore";

export type AppStore = {
  /** The backend state, or `null` before the first read. */
  state: AppState | null;
  /** True until the first state arrives (or fails). */
  loading: boolean;
  /** Why the first read failed, if it did. */
  error: string | null;
  /**
   * Icon cache-buster: starts at this page load's time and is bumped on
   * every `icons:updated`, so `<img>` URLs change when an icon does.
   */
  iconVersion: number;
  /** Reads the state and follows backend events; returns a stop function. */
  init: () => () => void;
  /** Replaces the state with one pushed by the backend. */
  applyState: (state: AppState) => void;
  bumpIconVersion: () => void;
  renameGroup: (id: string, name: string) => Promise<boolean>;
  setGroupApps: (id: string, appIds: string[]) => Promise<boolean>;
  setLaunchpadLayout: (layout: LaunchpadLayout) => Promise<boolean>;
};

/**
 * Applies `update` at once, then runs `call`. On failure the change is
 * rolled back (unless a newer backend state has replaced it meanwhile),
 * the raw error goes to the log and a friendly toast is shown.
 */
async function optimistic(
  get: () => AppStore,
  set: (partial: Partial<AppStore>) => void,
  update: (state: AppState) => AppState,
  call: () => Promise<Result<unknown>>,
  failure: string,
): Promise<boolean> {
  const previous = get().state;
  if (previous === null) return false;
  const next = update(previous);
  set({ state: next });
  const result = await call();
  if (result.ok) return true;
  if (get().state === next) set({ state: previous });
  console.error(`${failure} ${result.error}`);
  showToast({ kind: "error", title: failure, message: "Please try again." });
  return false;
}

export const useAppStore = create<AppStore>()((set, get) => ({
  state: null,
  loading: true,
  error: null,
  iconVersion: Date.now(),

  init: () => {
    // A pushed state is newer than the get_state snapshot, which may
    // resolve after it; once one has arrived, the snapshot is skipped.
    let pushed = false;
    const stopState = onEvent("state:changed", (state) => {
      pushed = true;
      get().applyState(state);
    });
    const stopIcons = onEvent("icons:updated", () => get().bumpIconVersion());
    void api.getState().then((result) => {
      if (pushed) return;
      if (result.ok) {
        get().applyState(result.value);
        return;
      }
      console.error(`get_state: ${result.error}`);
      set({ loading: false, error: result.error });
      showToast({
        kind: "error",
        title: "Couldn't load your groups.",
        message: "Restart Grouper to try again.",
      });
    });
    return () => {
      stopState();
      stopIcons();
    };
  },

  applyState: (state) => set({ state, loading: false, error: null }),

  bumpIconVersion: () => set((store) => ({ iconVersion: store.iconVersion + 1 })),

  renameGroup: (id, name) =>
    optimistic(
      get,
      set,
      (state) => withGroupRenamed(state, id, name),
      () => api.renameGroup(id, name),
      "Couldn't rename the group.",
    ),

  setGroupApps: (id, appIds) =>
    optimistic(
      get,
      set,
      (state) => withGroupApps(state, id, appIds),
      () => api.setGroupApps(id, appIds),
      "Couldn't save the new app order.",
    ),

  setLaunchpadLayout: (layout) =>
    optimistic(
      get,
      set,
      (state) => withLaunchpadLayout(state, layout),
      () => api.setLaunchpadLayout(layout),
      "Couldn't save the Launchpad layout.",
    ),
}));

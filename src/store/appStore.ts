import { create } from "zustand";
import type { AppState, LaunchpadLayout } from "../bindings";
import { api, type Result } from "../lib/api";
import { onEvent } from "../lib/events";
import { withGroupApps, withGroupOrder, withGroupRenamed, withLaunchpadLayout } from "./reducers";
import { showToast } from "./toastStore";

export type AppStore = {
  /** The backend state, or `null` before the first read. */
  state: AppState | null;
  /** True until the first state arrives (or fails). */
  loading: boolean;
  /** Why the first read failed, if it did. */
  error: string | null;
  /**
   * The icon cache-buster: starts at this page load's time, so a reloaded
   * page never reuses an earlier load's cached image, and is bumped on
   * every `icons:updated` (by `initIconUrls`), so `<img>` URLs change when
   * an icon does.
   */
  iconVersion: number;
  /** Reads the state and follows `state:changed`; returns a stop function. */
  init: () => () => void;
  /** Replaces the state with one pushed by the backend. */
  applyState: (state: AppState) => void;
  bumpIconVersion: () => void;
  renameGroup: (id: string, name: string) => Promise<boolean>;
  setGroupApps: (id: string, appIds: string[]) => Promise<boolean>;
  reorderGroups: (ids: string[]) => Promise<boolean>;
  setLaunchpadLayout: (layout: LaunchpadLayout) => Promise<boolean>;
};

/**
 * Bookkeeping for optimistic calls. `backend` is the last state the
 * backend sent (get_state or state:changed) and `applied` counts those, so
 * a refetch can tell whether a newer push beat it.
 */
/** How often, and how far apart, the first get_state is tried (about 3 s). */
const GET_STATE_ATTEMPTS = 15;
const GET_STATE_RETRY_MS = 200;

const flight = {
  backend: null as AppState | null,
  applied: 0,
  inFlight: 0,
  /** A call failed since the store last had nothing in flight. */
  failed: false,
  /** Two or more calls overlapped since then. */
  overlapped: false,
};

/**
 * Once no optimistic call is in flight and one of them failed: goes back
 * to the last backend state, so no failed change (and no other call's
 * optimistic state) is left showing. After overlapping calls the backend
 * may have taken some of them, so the state is fetched again; that read is
 * dropped if a state:changed arrives first.
 */
function settle(get: () => AppStore, set: (partial: Partial<AppStore>) => void): void {
  if (flight.inFlight > 0 || !flight.failed) return;
  const refetch = flight.overlapped;
  flight.failed = false;
  flight.overlapped = false;
  if (flight.backend !== null && get().state !== flight.backend) set({ state: flight.backend });
  if (!refetch) return;
  const applied = flight.applied;
  void api.getState().then((result) => {
    if (result.ok && flight.applied === applied && flight.inFlight === 0) {
      get().applyState(result.value);
    }
  });
}

/**
 * Applies `update` at once, then runs `call`. On failure the raw error
 * goes to the log, a friendly toast is shown and, once nothing else is in
 * flight, the store returns to the backend's state (see {@link settle}).
 * A newer state:changed always wins.
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
  flight.inFlight += 1;
  if (flight.inFlight > 1) flight.overlapped = true;
  set({ state: update(previous) });
  const result = await call();
  flight.inFlight -= 1;
  if (!result.ok) {
    flight.failed = true;
    console.error(`${failure} ${result.error}`);
    showToast({ kind: "error", title: failure, message: "Please try again." });
  }
  settle(get, set);
  return result.ok;
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
    let stopped = false;
    // A window can load before the backend has registered its state at
    // startup, so a failed first read is retried for a few seconds.
    const read = async (attempt: number): Promise<void> => {
      const result = await api.getState();
      if (pushed || stopped) return;
      if (result.ok) {
        get().applyState(result.value);
        return;
      }
      if (attempt < GET_STATE_ATTEMPTS) {
        await new Promise((resolve) => setTimeout(resolve, GET_STATE_RETRY_MS));
        return read(attempt + 1);
      }
      console.error(`get_state: ${result.error}`);
      set({ loading: false, error: result.error });
      showToast({
        kind: "error",
        title: "Couldn't load your groups.",
        message: "Restart Grouper to try again.",
      });
    };
    void read(1);
    return () => {
      stopped = true;
      stopState();
    };
  },

  applyState: (state) => {
    flight.backend = state;
    flight.applied += 1;
    set({ state, loading: false, error: null });
  },

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

  reorderGroups: (ids) =>
    optimistic(
      get,
      set,
      (state) => withGroupOrder(state, ids),
      () => api.reorderGroups(ids),
      "Couldn't save the new group order.",
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

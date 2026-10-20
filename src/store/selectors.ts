import type { AppEntry, AppState, Group } from "../bindings";

/** Most app icons a folder shows, like the taskbar `.ico` (S2-01). */
const MAX_FOLDER_ICONS = 4;

/**
 * Caches one derived value per `AppState` object. The store replaces the
 * state object on every change, so a cached value is never stale, and the
 * same state always gives the same object (safe for React selectors).
 */
function perState<T>(derive: (state: AppState) => T): (state: AppState) => T {
  const cache = new WeakMap<AppState, T>();
  return (state) => {
    const hit = cache.get(state);
    if (hit !== undefined) return hit;
    const value = derive(state);
    cache.set(state, value);
    return value;
  };
}

/** Apps the user has not hidden, in list order. Missing apps are kept. */
export const visibleApps = perState((state): AppEntry[] =>
  (state.apps ?? []).filter((app) => app.hidden !== true),
);

/** Every app by id. */
export const appsById = perState(
  (state): ReadonlyMap<string, AppEntry> =>
    new Map((state.apps ?? []).map((app) => [app.id ?? "", app])),
);

/** Every group by id. */
export const groupsById = perState(
  (state): ReadonlyMap<string, Group> =>
    new Map((state.groups ?? []).map((group) => [group.id ?? "", group])),
);

/**
 * The apps of group `groupId` in group order: ids that name a known,
 * non-hidden app. Empty for an unknown group. Missing apps are kept (they
 * show a badge).
 */
export function groupApps(state: AppState, groupId: string): AppEntry[] {
  const group = groupsById(state).get(groupId);
  if (group === undefined) return [];
  const byId = appsById(state);
  return (group.appIds ?? []).flatMap((id) => {
    const app = byId.get(id);
    return app !== undefined && app.hidden !== true ? [app] : [];
  });
}

/**
 * The apps a folder tile shows: the first four of the group's apps that
 * are neither missing nor hidden. Same rule as the taskbar `.ico`
 * (`group_icon_inputs` in Rust).
 */
export function folderIconApps(state: AppState, groupId: string): AppEntry[] {
  return groupApps(state, groupId)
    .filter((app) => app.missing !== true)
    .slice(0, MAX_FOLDER_ICONS);
}

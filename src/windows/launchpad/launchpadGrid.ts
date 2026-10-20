import type { AppState, LaunchpadItem } from "../../bindings";
import { DEFAULT_DENSITY, type GridDensity } from "../../lib/grid";
import { indexItems, type Indexed } from "../../lib/search";
import { appsById, groupApps, groupsById } from "../../store/selectors";

/**
 * The Launchpad's grid setting and icon size. Settings has no "Launchpad
 * grid" or "Icon size" field yet, so these are the DESIGN.md defaults
 * (7 x 5, Medium 72px), the same as `GRID_COLUMNS` and `GRID_ROWS` in Rust.
 */
export function launchpadDensity(): GridDensity {
  return DEFAULT_DENSITY;
}

/** A stable React key for an item. */
export function itemKey(item: LaunchpadItem): string {
  return item.kind === "app" ? `app:${item.appId}` : `folder:${item.groupId}`;
}

/** Caches one value per state object (the store replaces it on every change). */
function perState<T>(derive: (state: AppState) => T): (state: AppState | null) => T {
  const cache = new WeakMap<AppState, T>();
  let empty: T | undefined;
  return (state) => {
    if (state === null) return (empty ??= derive({}));
    const hit = cache.get(state);
    if (hit !== undefined) return hit;
    const value = derive(state);
    cache.set(state, value);
    return value;
  };
}

/**
 * The stored pages, skipping apps the user has hidden and ids the state
 * does not know. Pages may be longer than a screen's grid; see
 * `reflowPages`.
 */
export const visiblePages = perState((state): LaunchpadItem[][] => {
  const apps = appsById(state);
  const groups = groupsById(state);
  return (state.launchpad?.pages ?? []).map((page) =>
    page.filter((item) => {
      if (item.kind === "folder") return groups.has(item.groupId);
      const app = apps.get(item.appId);
      return app !== undefined && app.hidden !== true;
    }),
  );
});

/**
 * Every visible item with its normalised names, built once per state so
 * a keystroke only ranks. A folder carries the names of its apps.
 */
export const searchIndex = perState((state): Indexed<LaunchpadItem>[] => {
  const apps = appsById(state);
  const groups = groupsById(state);
  return indexItems<LaunchpadItem>(
    visiblePages(state)
      .flat()
      .map((item) =>
        item.kind === "app"
          ? { value: item, name: apps.get(item.appId)?.name ?? "" }
          : {
              value: item,
              name: groups.get(item.groupId)?.name ?? "",
              members: groupApps(state, item.groupId).map((app) => app.name ?? ""),
            },
      ),
  );
});

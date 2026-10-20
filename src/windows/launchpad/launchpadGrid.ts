import type { AppState, LaunchpadItem } from "../../bindings";
import type { GridDensity } from "../../lib/grid";
import { indexItems, type Indexed } from "../../lib/search";
import { appsById, groupApps, groupsById } from "../../store/selectors";
import { currentSettings, ICON_PX, useSetting, type FullSettings } from "../../store/settings";

/**
 * The Launchpad's grid setting and icon size now (7 x 5, Medium 72px by
 * default, as in Rust). Outside React; components use
 * {@link useLaunchpadDensity} so they follow a change.
 */
export function launchpadDensity(): GridDensity {
  const settings = currentSettings();
  return densityFor(settings.gridColumns, settings.gridRows, settings.iconSize);
}

/** {@link launchpadDensity} that re-renders when the settings change. */
export function useLaunchpadDensity(): GridDensity {
  const columns = useSetting("gridColumns");
  const rows = useSetting("gridRows");
  const iconSize = useSetting("iconSize");
  return densityFor(columns, rows, iconSize);
}

const memo = new Map<string, GridDensity>();

/**
 * The "Launchpad grid" and "Icon size" settings as a grid density: one
 * object per value, so it is stable across renders.
 */
function densityFor(columns: number, rows: number, iconSize: FullSettings["iconSize"]) {
  const key = `${columns}x${rows}:${iconSize}`;
  let density = memo.get(key);
  if (density === undefined) {
    density = { columns, rows, iconPx: ICON_PX[iconSize].launchpad };
    memo.set(key, density);
  }
  return density;
}

/**
 * Items a stored page holds: the grid setting, whatever the screen. A
 * move that overfills a stored page pushes the rest onto the next one.
 */
export function storedPerPage(): number {
  const density = launchpadDensity();
  return density.columns * density.rows;
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
 * Says whether a stored item shows: not an app the user has hidden, and
 * not an id the state does not know.
 */
export const visibleItem = perState((state): ((item: LaunchpadItem) => boolean) => {
  const apps = appsById(state);
  const groups = groupsById(state);
  return (item) => {
    if (item.kind === "folder") return groups.has(item.groupId);
    const app = apps.get(item.appId);
    return app !== undefined && app.hidden !== true;
  };
});

/**
 * The stored pages with only their visible items. Pages may be longer
 * than a screen's grid; see `reflowPages`.
 */
export const visiblePages = perState((state): LaunchpadItem[][] => {
  const visible = visibleItem(state);
  return (state.launchpad?.pages ?? []).map((page) => page.filter(visible));
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

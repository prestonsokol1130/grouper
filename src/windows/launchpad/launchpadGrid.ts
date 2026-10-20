import type { AppState, LaunchpadItem } from "../../bindings";
import { appsById, groupsById } from "../../store/selectors";

/**
 * The default "Launchpad grid" (DESIGN.md section 8), the same as
 * `GRID_COLUMNS` and `GRID_ROWS` in Rust. Settings has no grid field yet.
 */
export const GRID = { columns: 7, rows: 5 } as const;

/** Launchpad icons are Medium (DESIGN.md section 7). */
export const LAUNCHPAD_ICON_PX = 72;

/** A stable React key for an item. */
export function itemKey(item: LaunchpadItem): string {
  return item.kind === "app" ? `app:${item.appId}` : `folder:${item.groupId}`;
}

/**
 * The items on page `index`, skipping apps the user has hidden and ids
 * the state does not know, at most one grid's worth.
 */
export function pageItems(state: AppState | null, index: number): LaunchpadItem[] {
  if (state === null) return [];
  const page = state.launchpad?.pages?.[index] ?? [];
  const apps = appsById(state);
  const groups = groupsById(state);
  return page
    .filter((item) => {
      if (item.kind === "folder") return groups.has(item.groupId);
      const app = apps.get(item.appId);
      return app !== undefined && app.hidden !== true;
    })
    .slice(0, GRID.columns * GRID.rows);
}

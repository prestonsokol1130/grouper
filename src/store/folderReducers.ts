import type { AppState, Group, LaunchpadItem, LaunchpadSlot } from "../bindings";
import { findItem } from "../lib/layoutMap";
import { insertWithOverflow } from "../lib/layoutOps";

/**
 * Optimistic Launchpad folder changes, mirroring `state/folders.rs`. Each
 * returns the same object when the backend would refuse the change.
 */

/** The id a new folder has until the backend's state arrives. */
export const PENDING_FOLDER_ID = "pending:folder";

type Pages = LaunchpadItem[][];

const key = (item: LaunchpadItem) =>
  item.kind === "app" ? `app:${item.appId}` : `folder:${item.groupId}`;
const isApp = (item: LaunchpadItem, appId: string) => item.kind === "app" && item.appId === appId;

/** `pages` without App items for `appId`; a page this empties is removed. */
function withoutApp(pages: readonly (readonly LaunchpadItem[])[], appId: string): Pages {
  return pages.flatMap((page) => {
    const kept = page.filter((item) => !isApp(item, appId));
    return kept.length === 0 && page.length > 0 ? [] : [kept];
  });
}

function withPages(state: AppState, pages: Pages, groups = state.groups ?? []): AppState {
  return { ...state, groups, launchpad: { ...state.launchpad, pages } };
}

/** App `draggedAppId` dropped onto `target`: a new folder, or into one. */
export function withLaunchpadMerge(
  state: AppState,
  draggedAppId: string,
  target: LaunchpadItem,
): AppState {
  const groups = state.groups ?? [];
  const stored = state.launchpad?.pages ?? [];
  if (target.kind === "folder") {
    const group = groups.find((entry) => entry.id === target.groupId);
    if (group === undefined) return state;
    const appIds = group.appIds ?? [];
    const next = appIds.includes(draggedAppId)
      ? group
      : { ...group, appIds: [...appIds, draggedAppId] };
    const updated = groups.map((entry) => (entry === group ? next : entry));
    return withPages(state, withoutApp(stored, draggedAppId), updated);
  }
  const at = findItem(stored, key, key(target));
  if (target.appId === draggedAppId || at === null) return state;
  const folder: LaunchpadItem = { kind: "folder", groupId: PENDING_FOLDER_ID };
  const replaced = stored.map((page, index) =>
    index === at.page ? page.map((item, slot) => (slot === at.index ? folder : item)) : [...page],
  );
  const group: Group = { id: PENDING_FOLDER_ID, name: "", appIds: [target.appId, draggedAppId] };
  return withPages(state, withoutApp(replaced, draggedAppId), [...groups, group]);
}

/**
 * App `appId` taken out of folder `groupId` to stored slot `to`. A folder
 * left with one app or none is deleted, its remaining app in its place.
 */
export function withAppOutOfFolder(
  state: AppState,
  groupId: string,
  appId: string,
  to: LaunchpadSlot,
  perPage: number,
): AppState {
  const groups = state.groups ?? [];
  const group = groups.find((entry) => entry.id === groupId);
  if (group === undefined || !(group.appIds ?? []).includes(appId)) return state;
  const left = (group.appIds ?? []).filter((id) => id !== appId);
  const stored = state.launchpad?.pages ?? [];
  // Another tile of the app before `to` on its page shifts it left.
  const before = (stored[to.page] ?? []).slice(0, to.index).filter((item) => isApp(item, appId));
  const pages = stored.map((page) => page.filter((item) => !isApp(item, appId)));
  const hidden = new Set((state.apps ?? []).filter((app) => app.hidden).map((app) => app.id));
  const counts = (item: LaunchpadItem) => item.kind === "folder" || !hidden.has(item.appId);
  const slot = { page: to.page, index: to.index - before.length };
  const placed = insertWithOverflow(pages, { kind: "app", appId }, slot, perPage, counts);
  if (left.length > 1) {
    const kept = groups.map((entry) => (entry === group ? { ...group, appIds: left } : entry));
    return withPages(state, placed, kept);
  }
  const shown = new Set(placed.flat().map(key));
  const unfolded = placed.map((page) =>
    page.flatMap((item): LaunchpadItem[] => {
      if (item.kind !== "folder" || item.groupId !== groupId) return [item];
      const rest: LaunchpadItem[] = left.map((id) => ({ kind: "app", appId: id }));
      return rest.filter((entry) => !shown.has(key(entry)));
    }),
  );
  return withPages(
    state,
    unfolded,
    groups.filter((entry) => entry !== group),
  );
}

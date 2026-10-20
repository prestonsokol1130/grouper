import type { AppState, Group, LaunchpadLayout } from "../bindings";

/** `state` with group `id` changed by `change`; the same object if absent. */
function withGroup(state: AppState, id: string, change: (group: Group) => Group): AppState {
  const groups = state.groups ?? [];
  if (!groups.some((group) => group.id === id)) return state;
  return {
    ...state,
    groups: groups.map((group) => (group.id === id ? change(group) : group)),
  };
}

/** Optimistic rename: group `id` called `name` (trimmed, like the backend). */
export function withGroupRenamed(state: AppState, id: string, name: string): AppState {
  return withGroup(state, id, (group) => ({ ...group, name: name.trim() }));
}

/**
 * Optimistic reorder or edit of group `id`'s apps. Repeated ids keep their
 * first place and unknown ids are dropped, like the backend.
 */
export function withGroupApps(state: AppState, id: string, appIds: string[]): AppState {
  const known = new Set((state.apps ?? []).map((app) => app.id));
  const unique = [...new Set(appIds)].filter((appId) => known.has(appId));
  return withGroup(state, id, (group) => ({ ...group, appIds: unique }));
}

/** Optimistic Launchpad reorder: the whole layout replaced. */
export function withLaunchpadLayout(state: AppState, layout: LaunchpadLayout): AppState {
  return { ...state, launchpad: layout };
}

import { useMemo } from "react";
import type { AppEntry, Group } from "../bindings";
import { useAppStore } from "./appStore";
import { appsById, folderIconApps, groupsById } from "./selectors";

/** App `appId`, or `undefined` while loading or when it does not exist. */
export function useApp(appId: string): AppEntry | undefined {
  return useAppStore((store) => (store.state ? appsById(store.state).get(appId) : undefined));
}

/** Group `groupId`, or `undefined` while loading or when it does not exist. */
export function useGroup(groupId: string): Group | undefined {
  return useAppStore((store) => (store.state ? groupsById(store.state).get(groupId) : undefined));
}

/** The apps group `groupId`'s folder tile shows (at most four). */
export function useFolderIconApps(groupId: string): AppEntry[] {
  const state = useAppStore((store) => store.state);
  return useMemo(() => (state ? folderIconApps(state, groupId) : []), [state, groupId]);
}

/** The icon cache-buster. */
export function useIconVersion(): number {
  return useAppStore((store) => store.iconVersion);
}

import type { AppState, Settings } from "../bindings";
import { useAppStore } from "./appStore";

/** Every setting, filled in. */
export type FullSettings = Required<Settings>;

/**
 * The DESIGN.md section 9 defaults, the same as `Settings::default` in
 * Rust. Used for any field the backend has not sent (or before it has).
 */
export const DEFAULT_SETTINGS: FullSettings = {
  hotkey: "Alt+`",
  hotCorners: { topLeft: true, topRight: true },
  autostart: false,
  theme: "system",
  firstRunDone: false,
  accent: "windows",
  background: "mixed",
  folderShape: "square",
  iconSize: "medium",
  gridColumns: 7,
  gridRows: 5,
  animation: "genie",
  flyoutLayout: "3x3",
  flyoutIconSize: "matchTaskbar",
  showPageArrows: false,
  alwaysShowFocus: false,
};

/** "Launchpad grid" limits, as `GRID_COLUMNS` and `GRID_ROWS` in Rust. */
export const GRID_LIMITS = {
  columns: { min: 4, max: 10 },
  rows: { min: 3, max: 8 },
} as const;

/** DESIGN.md section 7 icon sizes in px ("Launchpad icon size"). */
export const ICON_PX = {
  small: { launchpad: 56 },
  medium: { launchpad: 72 },
  large: { launchpad: 96 },
} as const;

const cache = new WeakMap<AppState, FullSettings>();

/** `state`'s settings with every missing field at its default. */
export function settingsOf(state: AppState | null): FullSettings {
  if (state === null) return DEFAULT_SETTINGS;
  const hit = cache.get(state);
  if (hit !== undefined) return hit;
  const saved = state.settings ?? {};
  const settings: FullSettings = {
    ...DEFAULT_SETTINGS,
    ...Object.fromEntries(Object.entries(saved).filter(([, value]) => value != null)),
    hotCorners: { ...DEFAULT_SETTINGS.hotCorners, ...saved.hotCorners },
  };
  cache.set(state, settings);
  return settings;
}

/** The current settings, outside React. */
export function currentSettings(): FullSettings {
  return settingsOf(useAppStore.getState().state);
}

/** The current settings; re-renders on every state change. */
export function useSettings(): FullSettings {
  return useAppStore((store) => settingsOf(store.state));
}

/** One setting; re-renders only when that value changes. */
export function useSetting<K extends keyof FullSettings>(key: K): FullSettings[K] {
  return useAppStore((store) => settingsOf(store.state)[key]);
}

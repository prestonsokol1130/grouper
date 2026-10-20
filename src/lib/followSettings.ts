import { useAppStore } from "../store/appStore";
import { settingsOf } from "../store/settings";
import { onEvent } from "./events";
import { applyMaterial, setCustomAccent } from "./theme";

/**
 * Applies the settings this window paints itself, now and on every
 * change: the "Accent color" (theme.ts) and, after Rust swapped the
 * "Window background", the window's new material. Returns a stop
 * function.
 */
export function followSettings(): () => void {
  let last: string | undefined;
  const apply = () => {
    const accent = settingsOf(useAppStore.getState().state).accent;
    if (accent === last) return;
    last = accent;
    setCustomAccent(accent === "windows" ? null : accent);
  };
  apply();
  const stopStore = useAppStore.subscribe(apply);
  const stopMaterial = onEvent("material:changed", () => void applyMaterial());
  return () => {
    stopStore();
    stopMaterial();
  };
}

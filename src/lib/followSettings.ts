import { useAppStore } from "../store/appStore";
import { settingsOf } from "../store/settings";
import { onEvent } from "./events";
import { applyAlwaysShowFocus } from "./focusRing";
import { applyMaterial, setCustomAccent } from "./theme";

/**
 * Applies the settings this window paints itself, now and on every
 * change: the "Accent color" (theme.ts), "Always show keyboard focus"
 * (focusRing.ts) and, after Rust swapped the "Window background", the
 * window's new material. Returns a stop function.
 */
export function followSettings(): () => void {
  let last: string | undefined;
  const apply = () => {
    const settings = settingsOf(useAppStore.getState().state);
    applyAlwaysShowFocus(settings.alwaysShowFocus);
    const accent = settings.accent;
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

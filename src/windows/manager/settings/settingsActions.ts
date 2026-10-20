import type { Place, SettingsPatch } from "../../../bindings";
import { api } from "../../../lib/api";
import { clearHotkeyFailed } from "../../../store/hotkeyStatus";
import { showToast } from "../../../store/toastStore";

/** Logs `error` and shows a "what happened, what next" toast. */
function fail(what: string, error: string, title: string): void {
  console.error(`settings: ${what}: ${error}`);
  showToast({ kind: "error", title, message: "Please try again." });
}

/**
 * Saves the settings in `patch`. Every window hears the change through
 * `state:changed` and applies it live. Returns whether it worked.
 */
export async function saveSettings(patch: SettingsPatch): Promise<boolean> {
  const result = await api.updateSettings(patch);
  if (result.ok) return true;
  fail(`update ${Object.keys(patch).join(", ")}`, result.error, "Couldn't save that setting.");
  return false;
}

/**
 * Registers `hotkey` as the Launchpad shortcut. On failure (almost always
 * another app holding it) the old shortcut stays and the raw error goes
 * to the log. Returns whether it worked.
 */
export async function saveHotkey(hotkey: string): Promise<boolean> {
  const result = await api.setHotkey(hotkey);
  if (result.ok) {
    clearHotkeyFailed();
    return true;
  }
  console.warn(`settings: set hotkey ${hotkey}: ${result.error}`);
  return false;
}

/** Turns "Start with Windows" on or off. Returns whether it worked. */
export async function saveAutostart(enabled: boolean): Promise<boolean> {
  const result = await api.setAutostart(enabled);
  if (result.ok) return true;
  fail(`autostart ${enabled}`, result.error, "Couldn't change Start with Windows.");
  return false;
}

const PLACE_ERRORS: Record<Place, string> = {
  logs: "Couldn't open the logs folder.",
  data: "Couldn't open the data folder.",
  repo: "Couldn't open the Grouper page.",
};

/** Opens the logs folder, the data folder or the repo page. */
export async function openPlace(place: Place): Promise<void> {
  const result = await api.openPlace(place);
  if (!result.ok) fail(`open ${place}`, result.error, PLACE_ERRORS[place]);
}

/** Scans the Start Menu; the number of apps found, or `null` on failure. */
export async function rescanCount(): Promise<number | null> {
  const result = await api.rescanApps();
  if (result.ok) return result.value;
  fail("rescan", result.error, "Couldn't scan the Start Menu.");
  return null;
}

/** "1 app", "42 apps". */
export function appsFound(count: number): string {
  return count === 1 ? "1 app" : `${count} apps`;
}

import { showToast, useToastStore } from "../store/toastStore";
import { onEvent, type HotkeyFailed } from "./events";

/** A stored shortcut as DESIGN.md writes it: `Alt+\`` becomes `Alt + \``. */
export function displayHotkey(hotkey: string): string {
  return hotkey
    .split("+")
    .map((part) => part.trim())
    .join(" + ");
}

/** The DESIGN.md section 13 toast for a shortcut another app holds. */
function hotkeyFailedToast(failed: HotkeyFailed) {
  return {
    kind: "error" as const,
    title: `${displayHotkey(failed.hotkey)} is already used by another app.`,
    message: "Pick a different shortcut in Settings.",
  };
}

/**
 * Shows a toast whenever the backend says the Launchpad shortcut could not
 * be registered. The raw error goes to the log; the same toast is never
 * shown twice at once. Returns a function that stops listening.
 */
export function followHotkeyFailures(): () => void {
  return onEvent("hotkey:failed", (failed) => {
    console.warn(`hotkey: ${failed.error}`);
    const toast = hotkeyFailedToast(failed);
    const showing = useToastStore.getState().toasts.some((t) => t.title === toast.title);
    if (!showing) showToast(toast);
  });
}

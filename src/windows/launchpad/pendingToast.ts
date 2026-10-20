import { showToast, type ToastInput } from "../../store/toastStore";

/**
 * A toast the Launchpad could not show because it was hiding or hidden,
 * kept for its next open. Launch All needs this: the apps it starts take
 * focus, so the Launchpad hides on blur before it learns which apps
 * failed. Only the newest such toast is kept, and only for CARRY_MS.
 */

/** How long (ms) a kept toast still shows on the next open. */
export const CARRY_MS = 60_000;

let visible = false;
let pending: { toast: ToastInput; at: number } | null = null;

/** The Launchpad was shown (true) or started hiding or was hidden (false). */
export function setLaunchpadVisible(shown: boolean): void {
  visible = shown;
}

/** Shows `toast` now if the Launchpad is showing, else keeps it for the next open. */
export function showOrKeep(toast: ToastInput, now: number = Date.now()): void {
  if (visible) showToast(toast);
  else pending = { toast, at: now };
}

/** The kept toast, if it is still recent, and forgets it either way. */
export function takePending(now: number = Date.now()): ToastInput | null {
  const kept = pending;
  pending = null;
  return kept !== null && now - kept.at <= CARRY_MS ? kept.toast : null;
}

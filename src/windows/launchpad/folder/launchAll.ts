import { api } from "../../../lib/api";
import { showToast, type ToastInput } from "../../../store/toastStore";
import { showOrKeep } from "../pendingToast";

/** "A", "A and B", "A, B and C" (DESIGN.md section 13 lists no comma before "and"). */
export function listNames(names: readonly string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
}

/**
 * The toast for apps a Launch All could not open, in the section 13
 * voice: "Couldn't open Spotify and Slack. They may have been moved or
 * uninstalled."
 */
export function launchFailedToast(names: readonly string[]): ToastInput {
  const one = names.length === 1;
  return {
    kind: "error",
    title: names.length === 0 ? "Couldn't open the apps." : `Couldn't open ${listNames(names)}.`,
    message: `${one ? "It" : "They"} may have been moved or uninstalled.`,
  };
}

/**
 * Opens every app of group `groupId` (launch_group starts them 150ms
 * apart). Returns true when every app started, so the caller hides the
 * Launchpad; otherwise a toast names the apps that did not open and the
 * Launchpad stays, so the toast can be read.
 */
export async function launchAll(groupId: string): Promise<boolean> {
  const result = await api.launchGroup(groupId);
  if (!result.ok) {
    console.error(`launchpad: launch group ${groupId}: ${result.error}`);
    showToast({ kind: "error", title: "Couldn't open the group.", message: "Please try again." });
    return false;
  }
  const { failed } = result.value;
  if (failed.length === 0) return true;
  console.error(`launchpad: launch group ${groupId}: failed ${failed.join(", ")}`);
  // The apps that did start usually take focus and hide the Launchpad
  // first; then the toast waits for its next open.
  showOrKeep(launchFailedToast(failed));
  return false;
}

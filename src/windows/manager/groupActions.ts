import { api } from "../../lib/api";
import { showToast } from "../../store/toastStore";
import { NEW_GROUP_NAME } from "./groupName";

/**
 * Creates an empty "New Group". Returns its id, or `null` after logging
 * the error and showing a toast. The store learns about the group from
 * the `state:changed` the backend sends.
 */
export async function createGroup(): Promise<string | null> {
  const result = await api.createGroup(NEW_GROUP_NAME);
  if (result.ok) return result.value.id ?? null;
  console.error(`manager: create group: ${result.error}`);
  showToast({ kind: "error", title: "Couldn't create the group.", message: "Please try again." });
  return null;
}

/**
 * Deletes group `id` (the backend also removes its taskbar shortcut).
 * Returns whether it worked; a failure is logged and shown as a toast.
 */
export async function deleteGroup(id: string): Promise<boolean> {
  const result = await api.deleteGroup(id);
  if (result.ok) return true;
  console.error(`manager: delete group ${id}: ${result.error}`);
  showToast({ kind: "error", title: "Couldn't delete the group.", message: "Please try again." });
  return false;
}

/** Logs `error` and shows a "what happened, what next" toast. */
function fail(what: string, error: string, title: string, message = "Please try again."): void {
  console.error(`manager: ${what}: ${error}`);
  showToast({ kind: "error", title, message });
}

/** Makes group `id`'s taskbar shortcut. Returns whether it worked. */
export async function createShortcut(id: string): Promise<boolean> {
  const result = await api.createGroupShortcut(id);
  if (result.ok) return true;
  fail(`create shortcut ${id}`, result.error, "Couldn't create the shortcut.");
  return false;
}

/** Opens Explorer with group `id`'s shortcut selected. Returns whether it worked. */
export async function revealShortcut(id: string): Promise<boolean> {
  const result = await api.revealGroupShortcut(id);
  if (result.ok) return true;
  fail(`reveal shortcut ${id}`, result.error, "Couldn't show the shortcut.");
  return false;
}

/**
 * Opens app `appId` from the Manager; a failure says so in a toast. With
 * `remove`, the toast offers to take the app out of the group.
 */
export async function openApp(appId: string, name: string, remove?: () => void): Promise<void> {
  const result = await api.launchApp(appId);
  if (result.ok) return;
  console.error(`manager: launch ${appId}: ${result.error}`);
  showToast({
    kind: "error",
    title: name ? `Couldn't open ${name}.` : "Couldn't open the app.",
    message: "It may have been moved or uninstalled.",
    action: remove && { label: "Remove from group", run: remove },
  });
}

/** Scans the Start Menu again. Returns whether it worked. */
export async function rescanStartMenu(): Promise<boolean> {
  const result = await api.rescanApps();
  if (result.ok) {
    const found = result.value === 1 ? "1 app" : `${result.value} apps`;
    showToast({ kind: "success", title: `Start Menu scanned. Found ${found}.` });
    return true;
  }
  fail("rescan", result.error, "Couldn't scan the Start Menu.");
  return false;
}

/**
 * Lets the user pick programs that aren't in the Start Menu. Returns the
 * ids of the apps added (empty when the dialog was cancelled or failed).
 */
export async function pickOtherPrograms(): Promise<string[]> {
  const result = await api.pickAndAddApps();
  if (result.ok) return result.value.flatMap((app) => (app.id === undefined ? [] : [app.id]));
  fail("pick programs", result.error, "Couldn't add that program.");
  return [];
}

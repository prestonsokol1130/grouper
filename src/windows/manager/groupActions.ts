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

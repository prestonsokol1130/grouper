/** The name a new group starts with (the ticket's "New Group"). */
export const NEW_GROUP_NAME = "New Group";

/** DESIGN.md section 13 voice: what is wrong and what to do. */
const EMPTY_NAME_MESSAGE = "Give the group a name.";

/** A checked group name: the trimmed name, or why it can't be used. */
export type NameCheck = { ok: true; name: string } | { ok: false; message: string };

/**
 * Checks a name typed into the rename field. Like the backend's
 * `clean_name`, it trims the name and rejects one that is empty.
 */
export function checkGroupName(raw: string): NameCheck {
  const name = raw.trim();
  return name === "" ? { ok: false, message: EMPTY_NAME_MESSAGE } : { ok: true, name };
}

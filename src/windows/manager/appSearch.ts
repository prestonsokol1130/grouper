import type { AppEntry } from "../../bindings";
import { initials, normalize } from "../../lib/search";

/**
 * Whether an app called `name` matches what was typed in the picker's
 * search box: a substring of the name, or (for a query with no spaces) of
 * its initials, ignoring case and accents (the Launchpad's rules, see
 * `lib/search.ts`). An empty query matches everything.
 */
export function matchesQuery(name: string, query: string): boolean {
  const needle = normalize(query);
  if (needle === "") return true;
  if (normalize(name).includes(needle)) return true;
  return !needle.includes(" ") && initials(name).includes(needle);
}

/** The apps the picker lists for `query`, sorted by name. */
export function pickerApps(apps: readonly AppEntry[], query: string): AppEntry[] {
  return apps
    .filter((app) => app.id !== undefined && matchesQuery(app.name ?? "", query))
    .sort((a, b) => (a.name ?? "").localeCompare(b.name ?? "", undefined, { sensitivity: "base" }));
}

/**
 * The picker selection after ticking or unticking `id`: a newly ticked
 * app goes to the end, so the selection's order is the order apps are
 * added in.
 */
export function toggleId(selection: readonly string[], id: string): string[] {
  return selection.includes(id) ? selection.filter((other) => other !== id) : [...selection, id];
}

/**
 * The group's new app list once the picker is done. `groupIds` is the
 * group's current list, `listed` the ids the picker could show (apps the
 * picker hides, such as hidden apps, are kept where they are) and
 * `selection` what is ticked. Apps that stay keep their order; new ones
 * are appended in the order they were ticked.
 */
export function applySelection(
  groupIds: readonly string[],
  listed: ReadonlySet<string>,
  selection: readonly string[],
): string[] {
  const chosen = new Set(selection);
  const kept = groupIds.filter((id) => !listed.has(id) || chosen.has(id));
  const present = new Set(kept);
  return [...kept, ...selection.filter((id) => !present.has(id))];
}

/**
 * `groupIds` with the apps the Manager shows (`shown`, a subset) put in
 * the order of `reordered`. Ids it does not show keep their slots.
 */
export function withShownOrder(
  groupIds: readonly string[],
  shown: readonly string[],
  reordered: readonly string[],
): string[] {
  const visible = new Set(shown);
  let next = 0;
  return groupIds.map((id) => (visible.has(id) ? (reordered[next++] ?? id) : id));
}

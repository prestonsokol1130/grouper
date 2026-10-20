/**
 * Name search shared by the Launchpad and the Manager's app picker. Pure:
 * names are normalised once into an index ({@link indexItems}) and every
 * keystroke only runs {@link rank} over that index.
 */

/** How well a name matches, best first. 0 is no match. */
export const SCORE = {
  exact: 5,
  prefix: 4,
  wordPrefix: 3,
  initials: 2,
  substring: 1,
  none: 0,
} as const;

const MARKS = /\p{M}+/gu;
const NOT_WORD = /[^\p{L}\p{N}]+/u;

/**
 * Lower case, without accents (NFD with the combining marks removed) and
 * with runs of white space collapsed, so "Café  Notes" gives "cafe notes".
 */
export function normalize(text: string): string {
  return text.normalize("NFD").replace(MARKS, "").toLowerCase().replace(/\s+/g, " ").trim();
}

/** The words of `name`, split on spaces and punctuation and at camelCase. */
function words(name: string): string[] {
  return name
    .replace(/([\p{Ll}\p{N}])(\p{Lu})/gu, "$1 $2")
    .split(NOT_WORD)
    .map(normalize)
    .filter((word) => word !== "");
}

/**
 * The first letter of each word in `name`, lower case and without
 * accents, so "Visual Studio Code" gives "vsc" and "OneDrive" gives "od".
 */
export function initials(name: string): string {
  return words(name)
    .map((word) => word.charAt(0))
    .join("");
}

/** A name prepared for matching. */
export type IndexedName = {
  /** The whole name, normalised. */
  readonly flat: string;
  /** The name from its second word on, one entry per word ("studio code", "code"). */
  readonly tails: readonly string[];
  readonly initials: string;
};

/** Prepares `name` for {@link scoreName}. */
export function indexName(name: string): IndexedName {
  const split = words(name);
  return {
    flat: normalize(name),
    tails: split.slice(1).map((_, i) => split.slice(i + 1).join(" ")),
    initials: split.map((word) => word.charAt(0)).join(""),
  };
}

/**
 * How `name` matches the normalised query `q` (see {@link SCORE}):
 * the whole name, its start, the start of a later word, its initials (a
 * query without spaces) or anywhere in it.
 */
export function scoreName(name: IndexedName, q: string): number {
  if (q === "") return SCORE.none;
  if (name.flat === q) return SCORE.exact;
  if (name.flat.startsWith(q)) return SCORE.prefix;
  if (name.tails.some((tail) => tail.startsWith(q))) return SCORE.wordPrefix;
  if (!q.includes(" ") && name.initials.includes(q)) return SCORE.initials;
  if (name.flat.includes(q)) return SCORE.substring;
  return SCORE.none;
}

/** Something to search: an app, or a folder with the names of its apps. */
export type Searchable<T> = {
  readonly value: T;
  readonly name: string;
  /** For a folder, the names of the apps inside it. */
  readonly members?: readonly string[];
};

/** A {@link Searchable} with its names prepared. Build once, rank often. */
export type Indexed<T> = {
  readonly value: T;
  readonly name: string;
  readonly own: IndexedName;
  readonly members: readonly IndexedName[];
};

/** Prepares `items` for {@link rank}. */
export function indexItems<T>(items: readonly Searchable<T>[]): Indexed<T>[] {
  return items.map((item) => ({
    value: item.value,
    name: item.name,
    own: indexName(item.name),
    members: (item.members ?? []).map(indexName),
  }));
}

/**
 * The item's score for `q`, doubled so that at the same level a match on
 * the item's own name (odd) beats a match on an app inside a folder (even).
 */
function itemScore(item: Indexed<unknown>, q: string): number {
  const own = scoreName(item.own, q);
  if (own === SCORE.exact) return own * 2 + 1;
  let inside: number = SCORE.none;
  for (const member of item.members) {
    inside = Math.max(inside, scoreName(member, q));
    if (inside === SCORE.exact) break;
  }
  return Math.max(own * 2 + (own > 0 ? 1 : 0), inside * 2);
}

const byName = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

/**
 * The items that match `query`, best first: an exact name, then a name
 * that starts with it, then a later word that starts with it, then
 * initials ("vsc"), then anywhere in the name. A folder also matches
 * through the apps inside it, and is returned as the folder. Ties are
 * sorted by name. Case and accents are ignored. An empty query matches
 * nothing.
 */
export function rank<T>(query: string, items: readonly Indexed<T>[]): T[] {
  const q = normalize(query);
  if (q === "") return [];
  const hits: { item: Indexed<T>; score: number }[] = [];
  for (const item of items) {
    const score = itemScore(item, q);
    if (score > 0) hits.push({ item, score });
  }
  hits.sort((a, b) => b.score - a.score || byName.compare(a.item.name, b.item.name));
  return hits.map((hit) => hit.item.value);
}

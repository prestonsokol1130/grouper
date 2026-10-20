import { describe, expect, it } from "vitest";
import { reflowPages } from "./grid";
import { clampSlot, displayToStored, findItem, previewPages } from "./layoutMap";
import { moveItem } from "./layoutOps";

const key = (item: string) => item;
const visible = (item: string) => !item.startsWith("_");
const at = (page: number, index: number) => ({ page, index });

/** What the screen shows for `stored` at `perPage` a screen. */
const show = (stored: string[][], perPage: number) =>
  reflowPages(
    stored.map((page) => page.filter(visible)),
    perPage,
  );

/** Drops `id` on screen slot `drop` and returns the stored pages after. */
function drop(
  stored: string[][],
  perPage: number,
  id: string,
  target: { page: number; index: number },
) {
  const move = displayToStored(stored, visible, perPage, key, id, target);
  return move === null ? stored : moveItem(stored, move.from, move.to, 35, visible);
}

describe("displayToStored", () => {
  it("maps a move on one screen page to the stored page", () => {
    expect(drop([["a", "b", "c"]], 35, "a", at(0, 2))).toEqual([["b", "c", "a"]]);
    expect(drop([["a", "b", "c"]], 35, "c", at(0, 0))).toEqual([["c", "a", "b"]]);
  });

  it("keeps hidden apps where they were", () => {
    expect(drop([["a", "_h", "b", "c"]], 35, "c", at(0, 1))).toEqual([["a", "_h", "c", "b"]]);
    expect(drop([["a", "_h", "b"]], 35, "a", at(0, 1))).toEqual([["_h", "b", "a"]]);
  });

  it("maps a screen page that shows the second part of a stored page", () => {
    // Two a screen: stored page 0 shows as screens 0 (a b) and 1 (c d).
    const stored = [["a", "b", "c", "d"], ["e"]];
    expect(show(stored, 2)).toEqual([["a", "b"], ["c", "d"], ["e"]]);
    expect(drop(stored, 2, "a", at(1, 1))).toEqual([["b", "c", "a", "d"], ["e"]]);
    expect(drop(stored, 2, "e", at(1, 0))).toEqual([["a", "b", "e", "c", "d"]]);
    expect(drop(stored, 2, "d", at(2, 1))).toEqual([
      ["a", "b", "c"],
      ["e", "d"],
    ]);
  });

  it("drops at the end of a screen page after its last item", () => {
    expect(drop([["a", "b"], ["c"]], 35, "a", at(1, 9))).toEqual([["b"], ["c", "a"]]);
  });

  it("starts a new stored page on the page one past the last", () => {
    expect(drop([["a", "b"]], 35, "a", at(1, 0))).toEqual([["b"], ["a"]]);
  });

  it("is null when nothing moves or the item is unknown", () => {
    expect(displayToStored([["a", "b"]], visible, 35, key, "a", at(0, 0))).toBeNull();
    expect(displayToStored([["a"]], visible, 35, key, "a", at(0, 0))).toBeNull();
    expect(displayToStored([["a"]], visible, 35, key, "zz", at(0, 0))).toBeNull();
    expect(displayToStored([["a"]], visible, 35, key, "a", at(4, 0))).toBeNull();
  });
});

describe("previewPages", () => {
  it("shows the item in its slot, the page it left kept even when empty", () => {
    expect(previewPages([["a"], ["b"], []], key, "a", at(1, 0), 35)).toEqual([[], ["a", "b"], []]);
    expect(previewPages([["a"], []], key, "a", at(1, 0), 35)).toEqual([[], ["a"]]);
  });

  it("moves a full page's overflow on, as the drop will", () => {
    expect(previewPages([["a", "b"], ["c"]], key, "c", at(0, 9), 2)).toEqual([["a", "c"], ["b"]]);
  });
});

describe("helpers", () => {
  it("clamps a slot to a page's free slots", () => {
    expect(clampSlot(9, 3, 35)).toBe(3);
    expect(clampSlot(40, 35, 35)).toBe(34);
    expect(clampSlot(-1, 3, 35)).toBe(0);
  });

  it("finds a stored item", () => {
    expect(findItem([["a"], ["b", "c"]], key, "c")).toEqual(at(1, 1));
    expect(findItem([["a"]], key, "x")).toBeNull();
  });
});

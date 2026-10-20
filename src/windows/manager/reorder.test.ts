import { describe, expect, it } from "vitest";
import type { AppState } from "../../bindings";
import { withGroupOrder } from "../../store/reducers";
import { dropIndex, moveItem } from "./useDragReorder";

describe("moveItem", () => {
  it("moves one item and keeps the rest in order", () => {
    expect(moveItem(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    expect(moveItem(["a", "b", "c"], 1, 1)).toEqual(["a", "b", "c"]);
    expect(moveItem(["a", "b"], 5, 0)).toEqual(["a", "b"]);
  });
});

describe("dropIndex", () => {
  it("snaps to the nearest slot inside the list", () => {
    expect(dropIndex(0, 20, 48, 3)).toBe(0);
    expect(dropIndex(0, 30, 48, 3)).toBe(1);
    expect(dropIndex(0, 500, 48, 3)).toBe(2);
    expect(dropIndex(2, -500, 48, 3)).toBe(0);
    expect(dropIndex(1, 100, 0, 3)).toBe(1);
  });
});

describe("withGroupOrder", () => {
  const state: AppState = { groups: [{ id: "a" }, { id: "b" }, { id: "c" }] };

  it("reorders the groups optimistically", () => {
    const next = withGroupOrder(state, ["c", "a", "b"]);
    expect(next.groups?.map((group) => group.id)).toEqual(["c", "a", "b"]);
  });

  it("ignores anything that is not every group exactly once", () => {
    for (const ids of [["a", "b"], ["a", "a", "b"], ["a", "b", "x"], []]) {
      expect(withGroupOrder(state, ids)).toBe(state);
    }
  });
});

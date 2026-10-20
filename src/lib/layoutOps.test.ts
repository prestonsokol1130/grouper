import { describe, expect, it } from "vitest";
import { insertWithOverflow, moveItem } from "./layoutOps";

const at = (page: number, index: number) => ({ page, index });

describe("moveItem", () => {
  it("moves within a page like a list move, both ways", () => {
    const pages = [["a", "b", "c", "d"]];
    expect(moveItem(pages, at(0, 0), at(0, 2), 35)).toEqual([["b", "c", "a", "d"]]);
    expect(moveItem(pages, at(0, 3), at(0, 0), 35)).toEqual([["d", "a", "b", "c"]]);
    expect(pages).toEqual([["a", "b", "c", "d"]]);
  });

  it("moves to another page", () => {
    const pages = [
      ["a", "b"],
      ["c", "d"],
    ];
    expect(moveItem(pages, at(0, 0), at(1, 1), 35)).toEqual([["b"], ["c", "a", "d"]]);
    expect(moveItem(pages, at(1, 1), at(0, 0), 35)).toEqual([["d", "a", "b"], ["c"]]);
  });

  it("pushes a full page's overflow onto the next page, page after page", () => {
    const pages = [["a", "b", "c"], ["d", "e", "f"], ["g"]];
    expect(moveItem(pages, at(2, 0), at(0, 1), 3)).toEqual([
      ["a", "g", "b"],
      ["c", "d", "e"],
      ["f"],
    ]);
  });

  it("adds a page when the last page overflows", () => {
    expect(moveItem([["a", "b"], ["c"]], at(1, 0), at(0, 0), 2)).toEqual([["c", "a"], ["b"]]);
    expect(
      moveItem(
        [
          ["a", "b"],
          ["c", "d"],
        ],
        at(0, 0),
        at(1, 2),
        2,
      ),
    ).toEqual([["b"], ["c", "d"], ["a"]]);
  });

  it("opens a new page one past the last", () => {
    expect(moveItem([["a", "b"]], at(0, 1), at(1, 0), 35)).toEqual([["a"], ["b"]]);
  });

  it("removes the page a move empties and empty pages at the end", () => {
    expect(moveItem([["a"], ["b"]], at(0, 0), at(1, 1), 35)).toEqual([["b", "a"]]);
    expect(moveItem([["a"], ["b"], []], at(1, 0), at(0, 0), 35)).toEqual([["b", "a"]]);
    expect(moveItem([["a"], ["b"]], at(1, 0), at(2, 0), 35)).toEqual([["a"], ["b"]]);
  });

  it("does not count hidden items against the page size", () => {
    const visible = (item: string) => !item.startsWith("_");
    expect(moveItem([["a", "_h", "b"], ["c"]], at(1, 0), at(0, 0), 3, visible)).toEqual([
      ["c", "a", "_h", "b"],
    ]);
    expect(moveItem([["a", "b", "_h"], ["c"]], at(1, 0), at(0, 0), 2, visible)).toEqual([
      ["c", "a"],
      ["b", "_h"],
    ]);
  });

  it("leaves the pages alone for an unknown source", () => {
    expect(moveItem([["a"]], at(3, 0), at(0, 0), 35)).toEqual([["a"]]);
    expect(moveItem([["a"]], at(0, 5), at(0, 0), 35)).toEqual([["a"]]);
  });
});

describe("insertWithOverflow", () => {
  it("clamps the slot to the page", () => {
    expect(insertWithOverflow([["a"]], "b", at(0, 9), 35)).toEqual([["a", "b"]]);
    expect(insertWithOverflow([["a"]], "b", at(0, -2), 35)).toEqual([["b", "a"]]);
  });
});

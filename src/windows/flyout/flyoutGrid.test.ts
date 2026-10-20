import { describe, expect, it } from "vitest";
import { nextIndex, pagedMove } from "./flyoutGrid";

describe("nextIndex", () => {
  // Five tiles, three across:  0 1 2 / 3 4
  const move = (key: string, from: number) => nextIndex(key, from, 5, 3);

  it("walks left and right in reading order, stopping at the ends", () => {
    expect(move("ArrowRight", 2)).toBe(3);
    expect(move("ArrowLeft", 3)).toBe(2);
    expect(move("ArrowLeft", 0)).toBe(0);
    expect(move("ArrowRight", 4)).toBe(4);
  });

  it("keeps the column up and down", () => {
    expect(move("ArrowDown", 1)).toBe(4);
    expect(move("ArrowUp", 4)).toBe(1);
    expect(move("ArrowUp", 2)).toBe(2);
  });

  it("lands on the last tile from above a short last row", () => {
    expect(move("ArrowDown", 2)).toBe(4);
    expect(move("ArrowDown", 4)).toBe(4);
  });

  it("jumps with Home and End and ignores other keys", () => {
    expect(move("End", 0)).toBe(4);
    expect(move("Home", 4)).toBe(0);
    expect(move("a", 1)).toBeNull();
    expect(nextIndex("ArrowRight", 0, 0, 3)).toBeNull();
  });
});

describe("pagedMove", () => {
  // A full 3 x 3 page:  0 1 2 / 3 4 5 / 6 7 8, page 2 of 3.
  const move = (key: string, from: number, page = 1, count = 9) =>
    pagedMove(key, from, count, 3, page, 3);

  it("moves inside the page like nextIndex", () => {
    expect(move("ArrowRight", 0)).toEqual({ step: 0, index: 1 });
    expect(move("ArrowDown", 1)).toEqual({ step: 0, index: 4 });
    expect(move("ArrowLeft", 4)).toEqual({ step: 0, index: 3 });
  });

  it("goes on to the next page past the right edge, keeping the row", () => {
    expect(move("ArrowRight", 2)).toEqual({ step: 1, index: 0 });
    expect(move("ArrowRight", 5)).toEqual({ step: 1, index: 3 });
    expect(move("ArrowRight", 8)).toEqual({ step: 1, index: 6 });
  });

  it("goes back a page past the left edge, to the row's last column", () => {
    expect(move("ArrowLeft", 0)).toEqual({ step: -1, index: 2 });
    expect(move("ArrowLeft", 6)).toEqual({ step: -1, index: 8 });
  });

  it("turns from the last tile of a short page", () => {
    // Last page with 4 tiles: 0 1 2 / 3.
    expect(pagedMove("ArrowRight", 3, 4, 3, 1, 3)).toEqual({ step: 1, index: 3 });
  });

  it("stops at the first and last pages", () => {
    expect(move("ArrowLeft", 0, 0)).toBeNull();
    expect(move("ArrowRight", 2, 2)).toBeNull();
    expect(move("PageUp", 4, 0)).toBeNull();
    expect(move("PageDown", 4, 2)).toBeNull();
  });

  it("turns with Page Up and Page Down, keeping the slot", () => {
    expect(move("PageDown", 4)).toEqual({ step: 1, index: 4 });
    expect(move("PageUp", 4)).toEqual({ step: -1, index: 4 });
  });

  it("does nothing for keys that do not move", () => {
    expect(move("ArrowUp", 1)).toBeNull();
    expect(move("a", 1)).toBeNull();
    expect(pagedMove("ArrowRight", 0, 0, 3, 0, 1)).toBeNull();
  });
});

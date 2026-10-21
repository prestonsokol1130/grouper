import { describe, expect, it } from "vitest";
import { flyoutColumns, nextIndex } from "./flyoutGrid";

describe("flyoutColumns", () => {
  it("is up to three, matching the Rust layout", () => {
    expect([0, 1, 2, 3, 4, 9].map(flyoutColumns)).toEqual([3, 1, 2, 3, 3, 3]);
  });
});

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

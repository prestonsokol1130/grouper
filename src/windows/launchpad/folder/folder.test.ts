import { describe, expect, it } from "vitest";
import { computeGrid } from "../../../lib/grid";
import { screenToStoredSlot } from "../../../lib/layoutMap";
import { folderGrid, folderPages, slotAt } from "./folderLayout";
import { launchFailedToast, listNames } from "./launchAll";
import { originEdge } from "./useFolderMotion";

const MAIN = computeGrid({ width: 1920, height: 1080 });

describe("folder grid", () => {
  it("is two columns and rows smaller than the Launchpad, at least 3 x 2", () => {
    expect(folderGrid(40, MAIN)).toMatchObject({ columns: 5, rows: 3, perPage: 15 });
    expect(folderGrid(2, MAIN)).toMatchObject({ columns: 3, rows: 1 });
    expect(folderGrid(7, MAIN)).toMatchObject({ columns: 5, rows: 2 });
  });

  it("pages apps that do not fit", () => {
    expect(folderPages([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
    expect(folderPages([], 15)).toEqual([[]]);
  });

  it("finds the cell under a point", () => {
    const grid = folderGrid(15, MAIN);
    const rect = { left: 100, top: 100 };
    expect(slotAt({ x: 101, y: 101 }, rect, grid)).toBe(0);
    const pitch = grid.cell.width + grid.gap.column;
    expect(slotAt({ x: 100 + pitch + 5, y: 101 }, rect, grid)).toBe(1);
    expect(slotAt({ x: 5000, y: 5000 }, rect, grid)).toBe(grid.perPage - 1);
  });
});

describe("dropping an app out of a folder", () => {
  const visible = (item: string) => !item.startsWith("hidden");
  it("lands before the item it is dropped on, counting hidden items", () => {
    expect(screenToStoredSlot([["a", "hidden1", "b"]], visible, 35, { page: 0, index: 1 })).toEqual(
      { page: 0, index: 2 },
    );
  });
  it("goes after the last item past the end of a page", () => {
    expect(screenToStoredSlot([["a", "b"], ["c"]], visible, 35, { page: 0, index: 9 })).toEqual({
      page: 0,
      index: 2,
    });
  });
  it("maps a screen page split from a long stored page", () => {
    expect(screenToStoredSlot([["a", "b", "c"]], visible, 2, { page: 1, index: 0 })).toEqual({
      page: 0,
      index: 2,
    });
  });
  it("starts a new page one past the last and refuses pages that do not exist", () => {
    expect(screenToStoredSlot([["a"]], visible, 35, { page: 1, index: 0 })).toEqual({
      page: 1,
      index: 0,
    });
    expect(screenToStoredSlot([["a"]], visible, 35, { page: 3, index: 0 })).toBeNull();
  });
});

describe("Launch All failures", () => {
  it("lists names the DESIGN.md way", () => {
    expect(listNames(["Spotify"])).toBe("Spotify");
    expect(listNames(["Spotify", "Slack"])).toBe("Spotify and Slack");
    expect(listNames(["A", "B", "C"])).toBe("A, B and C");
  });
  it("says it or they", () => {
    expect(launchFailedToast(["Spotify", "Slack"])).toMatchObject({
      title: "Couldn't open Spotify and Slack.",
      message: "They may have been moved or uninstalled.",
    });
    expect(launchFailedToast(["Spotify"]).message).toBe("It may have been moved or uninstalled.");
  });
});

describe("open origin", () => {
  const panel = { left: 100, top: 100, right: 300, bottom: 300 };
  it("names the side of the panel the tile is on", () => {
    expect(originEdge({ x: 200, y: 400 }, panel)).toBe("bottom");
    expect(originEdge({ x: 200, y: 50 }, panel)).toBe("top");
    expect(originEdge({ x: 50, y: 200 }, panel)).toBe("left");
    expect(originEdge({ x: 400, y: 200 }, panel)).toBe("right");
    expect(originEdge({ x: 200, y: 250 }, panel)).toBe("bottom");
  });
});

import { describe, expect, it } from "vitest";
import type { AppEntry } from "../../bindings";
import {
  applySelection,
  initials,
  matchesQuery,
  pickerApps,
  toggleId,
  withShownOrder,
} from "./appSearch";
import { moveItem } from "./useDragReorder";
import { nearestSlot } from "./useGridReorder";
import { visibleRange } from "./useWindowedRows";

describe("picker search", () => {
  it("matches a case-insensitive substring of the name", () => {
    expect(matchesQuery("Visual Studio Code", "studio")).toBe(true);
    expect(matchesQuery("Visual Studio Code", "STUDIO c")).toBe(true);
    expect(matchesQuery("Notepad", "  pad ")).toBe(true);
    expect(matchesQuery("Notepad", "paint")).toBe(false);
  });

  it("matches initials, so vsc finds Visual Studio Code", () => {
    expect(initials("Visual Studio Code")).toBe("vsc");
    expect(initials("OneDrive")).toBe("od");
    expect(initials("Microsoft Edge (Dev)")).toBe("med");
    expect(matchesQuery("Visual Studio Code", "vsc")).toBe(true);
    expect(matchesQuery("Visual Studio Code", "VSC")).toBe(true);
    expect(matchesQuery("Microsoft Visual Studio Code", "vsc")).toBe(true);
    expect(matchesQuery("Visual Studio Code", "vcs")).toBe(false);
    // Initials only count for a query without spaces.
    expect(matchesQuery("Visual Studio Code", "v s")).toBe(false);
  });

  it("matches everything for an empty query", () => {
    expect(matchesQuery("Anything", "")).toBe(true);
    expect(matchesQuery("Anything", "   ")).toBe(true);
  });

  it("lists the matching apps sorted by name", () => {
    const apps: AppEntry[] = [
      { id: "z", name: "zoom" },
      { id: "v", name: "Visual Studio Code" },
      { id: "a", name: "Audacity" },
      { name: "No id" },
    ];
    expect(pickerApps(apps, "").map((app) => app.id)).toEqual(["a", "v", "z"]);
    expect(pickerApps(apps, "vsc").map((app) => app.id)).toEqual(["v"]);
    expect(pickerApps(apps, "o").map((app) => app.id)).toEqual(["v", "z"]);
  });
});

describe("picker selection", () => {
  it("toggles an id, appending newly ticked ones", () => {
    expect(toggleId(["a", "b"], "c")).toEqual(["a", "b", "c"]);
    expect(toggleId(["a", "b", "c"], "b")).toEqual(["a", "c"]);
    expect(toggleId(toggleId(["a"], "b"), "b")).toEqual(["a"]);
  });

  it("keeps the group's order and appends new apps", () => {
    const listed = new Set(["a", "b", "c", "d", "e"]);
    expect(applySelection(["c", "a", "b"], listed, ["c", "a", "b", "e", "d"])).toEqual([
      "c",
      "a",
      "b",
      "e",
      "d",
    ]);
    // Unticked apps leave; the rest keep their places.
    expect(applySelection(["c", "a", "b"], listed, ["a", "b", "c"])).toEqual(["c", "a", "b"]);
    expect(applySelection(["c", "a", "b"], listed, ["b", "d"])).toEqual(["b", "d"]);
  });

  it("keeps apps the picker does not list", () => {
    const listed = new Set(["a", "b"]);
    expect(applySelection(["h", "a", "b"], listed, ["b"])).toEqual(["h", "b"]);
  });
});

describe("app reorder", () => {
  it("moves the shown apps and leaves hidden ones in their slots", () => {
    const shown = ["a", "b", "c"];
    const reordered = moveItem(shown, 0, 2);
    expect(reordered).toEqual(["b", "c", "a"]);
    expect(withShownOrder(["a", "h", "b", "c"], shown, reordered)).toEqual(["b", "h", "c", "a"]);
    expect(withShownOrder(["a", "b"], ["a", "b"], ["b", "a"])).toEqual(["b", "a"]);
  });

  it("drops a dragged tile on the nearest grid slot", () => {
    // Two rows of three, 96px apart.
    const slots = [0, 1, 2, 3, 4, 5].map((i) => ({ x: (i % 3) * 96, y: Math.floor(i / 3) * 96 }));
    expect(nearestSlot(slots, { x: 10, y: 5 })).toBe(0);
    expect(nearestSlot(slots, { x: 180, y: 20 })).toBe(2);
    expect(nearestSlot(slots, { x: 100, y: 120 })).toBe(4);
    expect(nearestSlot(slots, { x: 900, y: 900 })).toBe(5);
    expect(nearestSlot([], { x: 0, y: 0 })).toBe(0);
  });
});

describe("windowing", () => {
  it("renders only the rows near the viewport", () => {
    expect(visibleRange(0, 400, 40, 1000)).toEqual({ start: 0, end: 18 });
    expect(visibleRange(4000, 400, 40, 1000)).toEqual({ start: 92, end: 118 });
    expect(visibleRange(39_000, 400, 40, 1000)).toEqual({ start: 967, end: 993 });
    expect(visibleRange(39_800, 400, 40, 1000)).toEqual({ start: 987, end: 1000 });
  });
});

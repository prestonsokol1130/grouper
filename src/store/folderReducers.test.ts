import { describe, expect, it } from "vitest";
import type { AppState, LaunchpadItem } from "../bindings";
import { PENDING_FOLDER_ID, withAppOutOfFolder, withLaunchpadMerge } from "./folderReducers";

const a = (appId: string): LaunchpadItem => ({ kind: "app", appId });
const f = (groupId: string): LaunchpadItem => ({ kind: "folder", groupId });
const STATE: AppState = {
  apps: ["w", "e", "n", "x", "y", "c"].map((id) => ({ id, name: id })),
  groups: [{ id: "g", name: "Tools", appIds: ["x", "y"] }],
  launchpad: {
    pages: [
      [a("w"), a("n"), a("e")],
      [f("g"), a("c")],
    ],
  },
};
const pages = (state: AppState) => state.launchpad?.pages;

describe("withLaunchpadMerge", () => {
  it("puts a pending folder in the target's place", () => {
    const next = withLaunchpadMerge(STATE, "e", a("w"));
    expect(pages(next)?.[0]).toEqual([f(PENDING_FOLDER_ID), a("n")]);
    expect(next.groups?.at(-1)?.appIds).toEqual(["w", "e"]);
  });

  it("appends to a folder once and drops a page it empties", () => {
    const next = withLaunchpadMerge(
      { ...STATE, launchpad: { pages: [[a("w")], [f("g")], [a("x")]] } },
      "x",
      f("g"),
    );
    expect(next.groups?.[0]?.appIds).toEqual(["x", "y"]);
    expect(pages(next)).toEqual([[a("w")], [f("g")]]);
  });

  it("refuses what the backend refuses", () => {
    expect(withLaunchpadMerge(STATE, "w", a("w"))).toBe(STATE);
    expect(withLaunchpadMerge(STATE, "w", a("x"))).toBe(STATE);
    expect(withLaunchpadMerge(STATE, "w", f("zz"))).toBe(STATE);
  });
});

describe("withAppOutOfFolder", () => {
  it("keeps a folder with apps left", () => {
    const state = { ...STATE, groups: [{ id: "g", appIds: ["x", "y", "c"] }] };
    const next = withAppOutOfFolder(state, "g", "x", { page: 0, index: 1 }, 35);
    expect(next.groups?.[0]?.appIds).toEqual(["y", "c"]);
    expect(pages(next)?.[0]).toEqual([a("w"), a("x"), a("n"), a("e")]);
  });

  it("dissolves a folder left with one app into it, spilling a full page", () => {
    const next = withAppOutOfFolder(STATE, "g", "x", { page: 0, index: 0 }, 3);
    expect(next.groups).toEqual([]);
    expect(pages(next)).toEqual([
      [a("x"), a("w"), a("n")],
      [a("e"), a("y"), a("c")],
    ]);
  });

  it("refuses an app that is not in the folder", () => {
    expect(withAppOutOfFolder(STATE, "g", "w", { page: 0, index: 0 }, 35)).toBe(STATE);
  });
});

import { describe, expect, it } from "vitest";
import type { AppState } from "../bindings";
import { withGroupApps, withGroupRenamed, withLaunchpadLayout } from "./reducers";
import { appsById, folderIconApps, groupApps, groupsById, visibleApps } from "./selectors";

const app = (id: string, flags: { missing?: boolean; hidden?: boolean } = {}) => ({
  id,
  name: id.toUpperCase(),
  ...flags,
});

const STATE: AppState = {
  apps: [
    app("a"),
    app("b"),
    app("gone", { missing: true }),
    app("hid", { hidden: true }),
    app("c"),
    app("d"),
    app("e"),
  ],
  groups: [
    { id: "g1", name: "Work", appIds: ["gone", "a", "hid", "b", "nope", "c", "d", "e"] },
    { id: "g2", name: "Empty", appIds: [] },
  ],
  launchpad: { pages: [] },
};

describe("selectors", () => {
  it("visibleApps drops hidden apps and keeps missing ones", () => {
    expect(visibleApps(STATE).map((a) => a.id)).toEqual(["a", "b", "gone", "c", "d", "e"]);
  });

  it("appsById and groupsById index by id", () => {
    expect(appsById(STATE).get("c")?.name).toBe("C");
    expect(groupsById(STATE).get("g2")?.name).toBe("Empty");
    expect(groupsById(STATE).get("zzz")).toBeUndefined();
  });

  it("returns the same object for the same state", () => {
    expect(appsById(STATE)).toBe(appsById(STATE));
    expect(visibleApps(STATE)).toBe(visibleApps(STATE));
    expect(appsById({ ...STATE })).not.toBe(appsById(STATE));
  });

  it("groupApps keeps group order, drops hidden and unknown ids", () => {
    expect(groupApps(STATE, "g1").map((a) => a.id)).toEqual(["gone", "a", "b", "c", "d", "e"]);
    expect(groupApps(STATE, "unknown")).toEqual([]);
  });

  it("folderIconApps takes the first four that are neither missing nor hidden", () => {
    expect(folderIconApps(STATE, "g1").map((a) => a.id)).toEqual(["a", "b", "c", "d"]);
    expect(folderIconApps(STATE, "g2")).toEqual([]);
  });
});

describe("reducers", () => {
  it("renames a group, trimmed, without touching the input", () => {
    const next = withGroupRenamed(STATE, "g2", "  Games ");
    expect(groupsById(next).get("g2")?.name).toBe("Games");
    expect(groupsById(STATE).get("g2")?.name).toBe("Empty");
  });

  it("returns the same state for an unknown group", () => {
    expect(withGroupRenamed(STATE, "nope", "x")).toBe(STATE);
    expect(withGroupApps(STATE, "nope", ["a"])).toBe(STATE);
  });

  it("reorders group apps, dropping repeats and unknown ids", () => {
    const next = withGroupApps(STATE, "g2", ["b", "a", "b", "zzz"]);
    expect(groupsById(next).get("g2")?.appIds).toEqual(["b", "a"]);
  });

  it("replaces the Launchpad layout", () => {
    const layout = { pages: [[{ kind: "app" as const, appId: "a" }]] };
    expect(withLaunchpadLayout(STATE, layout).launchpad).toBe(layout);
  });
});

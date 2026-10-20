import { describe, expect, it } from "vitest";
import { invokeMock } from "../test/tauriMock";
import { api, errorText } from "./api";

describe("api wrappers", () => {
  it("maps a success to ok/value and passes the arguments", async () => {
    invokeMock.mockResolvedValueOnce({ id: "g1", name: "Work" });
    const result = await api.renameGroup("g1", "Work");
    expect(result).toEqual({ ok: true, value: { id: "g1", name: "Work" } });
    expect(invokeMock).toHaveBeenCalledWith("rename_group", { id: "g1", name: "Work" });
  });

  it("maps a Rust Err string to ok:false without throwing", async () => {
    invokeMock.mockRejectedValueOnce("Group not found: g9");
    await expect(api.deleteGroup("g9")).resolves.toEqual({
      ok: false,
      error: "Group not found: g9",
    });
  });

  it("maps a thrown Error (e.g. IPC failure) to ok:false", async () => {
    invokeMock.mockRejectedValueOnce(new Error("ipc down"));
    await expect(api.getState()).resolves.toEqual({ ok: false, error: "ipc down" });
  });

  it("wraps commands without a Rust Result too", async () => {
    invokeMock.mockResolvedValueOnce(null);
    await expect(api.getFlyout()).resolves.toEqual({ ok: true, value: null });
    invokeMock.mockRejectedValueOnce({ code: 3 });
    await expect(api.hideFlyout()).resolves.toEqual({ ok: false, error: '{"code":3}' });
  });

  it("passes camelCase argument names through", async () => {
    invokeMock.mockResolvedValueOnce({ id: "g1", appIds: ["a"] });
    await api.setGroupApps("g1", ["a"]);
    expect(invokeMock).toHaveBeenCalledWith("set_group_apps", { id: "g1", appIds: ["a"] });
  });

  it("wraps every command in the bindings", () => {
    expect(Object.keys(api).sort()).toEqual(
      [
        "addAppFromPath",
        "createGroup",
        "createGroupShortcut",
        "deleteGroup",
        "getAccentColor",
        "getCacheDir",
        "getFlyout",
        "getState",
        "getWindowMaterial",
        "hideFlyout",
        "launchApp",
        "launchFromFlyout",
        "launchGroup",
        "pickAndAddApps",
        "removeApp",
        "renameGroup",
        "reorderGroups",
        "rescanApps",
        "revealGroupShortcut",
        "setGroupApps",
        "setLaunchpadLayout",
        "updateSettings",
      ].sort(),
    );
  });
});

describe("errorText", () => {
  it("handles strings, errors, objects and the unserializable", () => {
    expect(errorText("plain")).toBe("plain");
    expect(errorText(new Error("boom"))).toBe("boom");
    expect(errorText({ a: 1 })).toBe('{"a":1}');
    const cyclic: Record<string, unknown> = {};
    cyclic.self = cyclic;
    expect(errorText(cyclic)).toBe("[object Object]");
    expect(errorText(undefined)).toBe("undefined");
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState } from "../bindings";
import { emitMock, invokeMock } from "../test/tauriMock";
import { useAppStore } from "./appStore";
import { groupsById } from "./selectors";
import { useToastStore } from "./toastStore";

const STATE: AppState = {
  apps: [
    { id: "a", name: "A" },
    { id: "b", name: "B" },
  ],
  groups: [{ id: "g1", name: "Work", appIds: ["a", "b"] }],
};

const store = () => useAppStore.getState();
const groupName = () => groupsById(store().state ?? {}).get("g1")?.name;

/** Lets pending promise callbacks run. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
  useAppStore.setState({ state: null, loading: true, error: null, iconVersion: 1 });
  useToastStore.setState({ toasts: [] });
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

describe("init", () => {
  it("loads get_state and follows state:changed", async () => {
    invokeMock.mockResolvedValueOnce(STATE);
    const stop = store().init();
    await flush();
    expect(invokeMock).toHaveBeenCalledWith("get_state", undefined);
    expect(store()).toMatchObject({ state: STATE, loading: false, error: null });

    const pushed = { ...STATE, groups: [] };
    emitMock("state:changed", pushed);
    expect(store().state).toBe(pushed);
    stop();
  });

  it("skips a get_state snapshot older than a pushed state", async () => {
    let resolve: (value: AppState) => void = () => undefined;
    invokeMock.mockReturnValueOnce(new Promise((r) => (resolve = r)));
    store().init();
    const pushed = { ...STATE, groups: [] };
    emitMock("state:changed", pushed);
    resolve(STATE);
    await flush();
    expect(store().state).toBe(pushed);
  });

  it("records a failed first read and shows a toast", async () => {
    invokeMock.mockRejectedValue("disk on fire");
    store().init();
    await flush();
    expect(store()).toMatchObject({ state: null, loading: false, error: "disk on fire" });
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(useToastStore.getState().toasts).toHaveLength(1);
  });
});

describe("optimistic updates", () => {
  beforeEach(() => store().applyState(STATE));

  it("renames at once and keeps it when the backend agrees", async () => {
    invokeMock.mockResolvedValueOnce({ id: "g1", name: "Games" });
    const pending = store().renameGroup("g1", "Games");
    expect(groupName()).toBe("Games");
    await expect(pending).resolves.toBe(true);
    expect(groupName()).toBe("Games");
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("rolls a rename back on error and shows a friendly toast", async () => {
    invokeMock.mockRejectedValueOnce("Group not found: g1");
    await expect(store().renameGroup("g1", "Games")).resolves.toBe(false);
    expect(store().state).toBe(STATE);
    const [toast] = useToastStore.getState().toasts;
    expect(toast).toMatchObject({ kind: "error", title: "Couldn't rename the group." });
    expect(toast?.message).not.toContain("g1");
  });

  it("rolls a reorder back on error", async () => {
    invokeMock.mockRejectedValueOnce("nope");
    const pending = store().setGroupApps("g1", ["b", "a"]);
    expect(groupsById(store().state ?? {}).get("g1")?.appIds).toEqual(["b", "a"]);
    await pending;
    expect(groupsById(store().state ?? {}).get("g1")?.appIds).toEqual(["a", "b"]);
  });

  it("keeps a newer backend state instead of rolling back over it", async () => {
    let reject: (reason: string) => void = () => undefined;
    invokeMock.mockReturnValueOnce(new Promise((_, r) => (reject = r)));
    const pending = store().renameGroup("g1", "Games");
    const newer = { ...STATE, groups: [{ id: "g1", name: "Server" }] };
    store().applyState(newer);
    reject("failed");
    await pending;
    expect(store().state).toBe(newer);
  });

  it("returns to the original state when two overlapping calls both fail", async () => {
    const rejects: ((reason: string) => void)[] = [];
    const pending = () => new Promise((_, reject) => rejects.push(reject));
    invokeMock.mockImplementation((command) =>
      command === "get_state" ? Promise.resolve(STATE) : pending(),
    );
    const rename = store().renameGroup("g1", "Games");
    const reorder = store().setGroupApps("g1", ["b", "a"]);
    expect(groupName()).toBe("Games");
    rejects[0]?.("rename failed");
    await rename;
    // The reorder is still in flight; the rename's failure waits for it.
    expect(groupsById(store().state ?? {}).get("g1")?.appIds).toEqual(["b", "a"]);
    rejects[1]?.("reorder failed");
    await reorder;
    expect(store().state).toBe(STATE);
    await flush();
    expect(store().state).toEqual(STATE);
    expect(invokeMock).toHaveBeenCalledWith("get_state", undefined);
  });

  it("refetches after overlap so a call that succeeded is kept", async () => {
    const server = { ...STATE, groups: [{ id: "g1", name: "Work", appIds: ["b", "a"] }] };
    let rejectRename: (reason: string) => void = () => undefined;
    invokeMock.mockImplementation((command) => {
      if (command === "rename_group") return new Promise((_, r) => (rejectRename = r));
      if (command === "set_group_apps") return Promise.resolve(server.groups[0]);
      return Promise.resolve(server);
    });
    const rename = store().renameGroup("g1", "Games");
    await store().setGroupApps("g1", ["b", "a"]);
    rejectRename("rename failed");
    await rename;
    await flush();
    expect(store().state).toBe(server);
  });

  it("ignores the refetch when a state:changed arrives first", async () => {
    let resolveFetch: (value: AppState) => void = () => undefined;
    const rejects: ((reason: string) => void)[] = [];
    invokeMock.mockImplementation((command) =>
      command === "get_state"
        ? new Promise((r) => (resolveFetch = r))
        : new Promise((_, reject) => rejects.push(reject)),
    );
    const a = store().renameGroup("g1", "One");
    const b = store().renameGroup("g1", "Two");
    rejects[0]?.("x");
    rejects[1]?.("y");
    await Promise.all([a, b]);
    const pushed = { ...STATE, groups: [] };
    store().applyState(pushed);
    resolveFetch(STATE);
    await flush();
    expect(store().state).toBe(pushed);
  });

  it("does nothing before the state has loaded", async () => {
    useAppStore.setState({ state: null });
    await expect(store().renameGroup("g1", "x")).resolves.toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("saves a Launchpad layout optimistically", async () => {
    invokeMock.mockResolvedValueOnce({ pages: [] });
    const layout = { pages: [[{ kind: "folder" as const, groupId: "g1" }]] };
    const pending = store().setLaunchpadLayout(layout);
    expect(store().state?.launchpad).toBe(layout);
    await expect(pending).resolves.toBe(true);
  });
});

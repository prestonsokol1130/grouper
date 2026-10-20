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
  it("loads get_state and follows state:changed and icons:updated", async () => {
    invokeMock.mockResolvedValueOnce(STATE);
    const stop = store().init();
    await flush();
    expect(invokeMock).toHaveBeenCalledWith("get_state", undefined);
    expect(store()).toMatchObject({ state: STATE, loading: false, error: null });

    const pushed = { ...STATE, groups: [] };
    emitMock("state:changed", pushed);
    expect(store().state).toBe(pushed);

    emitMock("icons:updated", ["a"]);
    expect(store().iconVersion).toBe(2);
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
    invokeMock.mockRejectedValueOnce("disk on fire");
    store().init();
    await flush();
    expect(store()).toMatchObject({ state: null, loading: false, error: "disk on fire" });
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

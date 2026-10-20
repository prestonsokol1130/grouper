import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, LaunchpadItem } from "../../bindings";
import { Toaster } from "../../components/Toaster";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";
import { MERGE_HOLD_MS } from "./mergeIntent";

const app = (appId: string): LaunchpadItem => ({ kind: "app", appId });
const STATE: AppState = {
  apps: ["a", "b", "c", "x"].map((id) => ({ id, name: `App ${id}` })),
  groups: [{ id: "g", name: "Tools", appIds: ["x"] }],
  launchpad: { pages: [[app("a"), app("b"), { kind: "folder", groupId: "g" }, app("c")]] },
};
const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

/** The cell the lifted tile came from; the drag overlay starts over it. */
let liftedFrom = 0;

/** Cells 100px apart in a row, 88px square, from x = 100 (as in LaunchpadReorder). */
function fakeLayout() {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const cell = this.closest("[data-sortable]");
    const lifted = this.closest(".launchpad-lifted") !== null;
    const parent = cell?.parentElement;
    const index = lifted ? liftedFrom : parent ? [...parent.children].indexOf(cell as Element) : 0;
    const size = cell || lifted ? 88 : 0;
    return DOMRect.fromRect({ x: 100 + index * 100, y: 100, width: size, height: size });
  });
}

/** The x of the middle of cell `index`; y 144 is the middle of the row. */
const mid = (index: number) => 144 + index * 100;
const Y = 144;

const cellOf = (name: string) =>
  document.querySelector(`[data-sortable] [aria-label="${name}"]`)?.closest("[data-sortable]");
const calls = (name: string) => invokeMock.mock.calls.filter(([command]) => command === name);

/** What the backend answers: the state for get_state, a group id for a merge. */
const reply = (command: string) =>
  command === "get_state" ? STATE : command === "apply_launchpad_merge" ? "g" : null;

const flush = () => act(() => Promise.resolve());
const advance = (ms: number) => act(() => vi.advanceTimersByTime(ms));
const move = (x: number) => act(() => fireEvent.pointerMove(document, { clientX: x, clientY: Y }));

/** Presses tile `name` at its middle and lifts it past the pick-up distance. */
async function lift(name: string, index: number) {
  const button = screen.getByRole("button", { name });
  liftedFrom = index;
  act(() =>
    fireEvent.pointerDown(button, { clientX: mid(index), clientY: Y, button: 0, isPrimary: true }),
  );
  move(mid(index) + 10);
  await flush();
}

const drop = async (x: number) => {
  act(() => fireEvent.pointerUp(document, { clientX: x, clientY: Y }));
  await flush();
};

beforeEach(async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  invokeMock.mockImplementation((command, args) =>
    Promise.resolve(
      command === "set_launchpad_layout" ? (args as { layout: unknown }).layout : reply(command),
    ),
  );
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  fakeLayout();
  render(
    <>
      <LaunchpadRoot />
      <Toaster />
    </>,
  );
  await flush();
  act(() => emitMock("launchpad:show", SHOW));
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("Launchpad drag to make folders", () => {
  it("merges an app into another after resting MERGE_HOLD_MS over its centre", async () => {
    await lift("App a", 0);
    move(mid(1));
    advance(MERGE_HOLD_MS - 1);
    expect(cellOf("App b")?.getAttribute("data-merge")).toBeNull();
    advance(1);
    expect(cellOf("App b")?.getAttribute("data-merge")).toBe("app");
    await drop(mid(1));
    expect(calls("apply_launchpad_merge").map(([, args]) => args)).toEqual([
      { draggedAppId: "a", target: { kind: "app", appId: "b" } },
    ]);
    expect(calls("set_launchpad_layout")).toHaveLength(0);
    // Optimistic: the folder took b's place and a's tile is gone.
    expect(useAppStore.getState().state?.launchpad?.pages?.[0]?.[0]).toEqual({
      kind: "folder",
      groupId: "pending:folder",
    });
  });

  it("adds an app to a folder it rests on", async () => {
    await lift("App c", 3);
    move(mid(2));
    advance(MERGE_HOLD_MS);
    expect(cellOf("Tools")?.getAttribute("data-merge")).toBe("folder");
    await drop(mid(2));
    expect(calls("apply_launchpad_merge").map(([, args]) => args)).toEqual([
      { draggedAppId: "c", target: { kind: "folder", groupId: "g" } },
    ]);
    expect(useAppStore.getState().state?.groups?.[0]?.appIds).toEqual(["x", "c"]);
  });

  it("drops on a tile's edge as a reorder, however long it rests", async () => {
    await lift("App a", 0);
    // Inside b's cell but left of its middle half.
    move(mid(1) - 30);
    advance(MERGE_HOLD_MS * 2);
    expect(document.querySelector("[data-merge]")).toBeNull();
    await drop(mid(1) - 30);
    expect(calls("apply_launchpad_merge")).toHaveLength(0);
    expect(calls("set_launchpad_layout")).toHaveLength(1);
  });

  it("goes back to reorder when the pointer leaves the centre before the drop", async () => {
    await lift("App a", 0);
    move(mid(1));
    advance(MERGE_HOLD_MS);
    move(mid(1) - 30);
    expect(document.querySelector("[data-merge]")).toBeNull();
    await drop(mid(1) - 30);
    expect(calls("apply_launchpad_merge")).toHaveLength(0);
  });

  it("never merges a dragged folder", async () => {
    await lift("Tools", 2);
    move(mid(1));
    advance(MERGE_HOLD_MS);
    expect(document.querySelector("[data-merge]")).toBeNull();
  });

  it("rolls back and says so when the merge fails", async () => {
    invokeMock.mockImplementation((command) =>
      command === "apply_launchpad_merge"
        ? Promise.reject(new Error("disk full"))
        : Promise.resolve(reply(command)),
    );
    await lift("App a", 0);
    move(mid(1));
    advance(MERGE_HOLD_MS);
    await drop(mid(1));
    await flush();
    expect(useAppStore.getState().state).toEqual(STATE);
    expect(screen.getByText("Couldn't make that folder.")).toBeTruthy();
  });
});

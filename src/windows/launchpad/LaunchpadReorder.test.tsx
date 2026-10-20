import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, LaunchpadItem, LaunchpadLayout } from "../../bindings";
import { Toaster } from "../../components/Toaster";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";

const item = (appId: string): LaunchpadItem => ({ kind: "app", appId });
const STATE: AppState = {
  apps: ["a", "b", "c", "hid", "d"].map((id) => ({
    id,
    name: `App ${id}`,
    hidden: id === "hid",
  })),
  groups: [],
  launchpad: { pages: [["a", "b", "hid", "c"].map(item), [item("d")]] },
};
const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

const flush = () => act(() => Promise.resolve());
const tile = (id: string) => screen.getByRole("button", { name: `App ${id}` });
/** The tile in the grid (a drag also draws one in the overlay). */
const sortable = (id: string) =>
  document.querySelector(`[data-sortable] [aria-label="App ${id}"]`) as Element;
const saved = () =>
  invokeMock.mock.calls
    .filter(([command]) => command === "set_launchpad_layout")
    .map(([, args]) => (args as { layout: LaunchpadLayout }).layout.pages);
const ids = (pages: LaunchpadItem[][] | undefined) =>
  pages?.map((page) => page.map((entry) => (entry.kind === "app" ? entry.appId : entry.groupId)));

/** Lays the sortable cells out 100px apart in a row, as a grid would. */
function fakeLayout() {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const cell = this.closest("[data-sortable]");
    const index = cell?.parentElement ? [...cell.parentElement.children].indexOf(cell) : 0;
    const left = cell ? 100 + index * 100 : 0;
    const size = cell ? 88 : 0;
    return DOMRect.fromRect({ x: left, y: 100, width: size, height: size });
  });
}

/** Lets the keyboard sensor attach (it waits a tick) and React settle. */
const tick = () => act(() => new Promise((resolve) => setTimeout(resolve, 0)));

async function key(target: Element, code: string, keyName: string) {
  fireEvent.keyDown(target, { code, key: keyName });
  await tick();
}

async function renderShown() {
  render(
    <>
      <LaunchpadRoot />
      <Toaster />
    </>,
  );
  await flush();
  act(() => emitMock("launchpad:show", SHOW));
}

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  invokeMock.mockImplementation((command, args) =>
    Promise.resolve(
      command === "set_launchpad_layout" ? (args as { layout: LaunchpadLayout }).layout : null,
    ),
  );
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  fakeLayout();
});

afterEach(() => vi.restoreAllMocks());

describe("Launchpad drag to reorder", () => {
  it("moves a tile with the keyboard and saves the stored order, hidden apps kept", async () => {
    await renderShown();
    const a = tile("a");
    a.focus();
    await key(a, "Space", " ");
    expect(screen.getByTestId("drop-slot")).toBeTruthy();
    await key(a, "ArrowRight", "ArrowRight");
    await key(a, "ArrowRight", "ArrowRight");
    await key(a, "Space", " ");
    await flush();
    expect(ids(saved()[0])).toEqual([["b", "hid", "c", "a"], ["d"]]);
    expect(ids(useAppStore.getState().state?.launchpad?.pages)).toEqual([
      ["b", "hid", "c", "a"],
      ["d"],
    ]);
    // Neither the drop nor the pick-up opened the app.
    expect(invokeMock.mock.calls.some(([command]) => command === "launch_app")).toBe(false);
  });

  it("carries the tile to the end of the page it turns to, or onto a new page", async () => {
    await renderShown();
    const pages = document.querySelector(".launchpad-pages") as Element;
    const wheel = (timeStamp: number, deltaY = 100) => {
      const event = new WheelEvent("wheel", { deltaY, bubbles: true });
      Object.defineProperty(event, "timeStamp", { value: timeStamp });
      fireEvent(pages, event);
    };
    const a = tile("a");
    a.focus();
    await key(a, "Space", " ");
    // The drag adds an empty page after the last.
    expect(screen.getAllByRole("button", { name: /^Page / })).toHaveLength(3);
    wheel(1000);
    await tick();
    await key(sortable("a"), "Space", " ");
    await flush();
    // It lands in front of "d", the tile it is now over.
    expect(ids(saved()[0])).toEqual([
      ["b", "hid", "c"],
      ["a", "d"],
    ]);

    wheel(2000, -100);
    const b = tile("b");
    b.focus();
    await key(b, "Space", " ");
    wheel(3000);
    wheel(5000);
    await tick();
    await key(sortable("b"), "Space", " ");
    await flush();
    expect(ids(saved()[1])).toEqual([["hid", "c"], ["a", "d"], ["b"]]);
  });

  it("puts the tile back on Escape without saving or closing", async () => {
    await renderShown();
    const b = tile("b");
    b.focus();
    await key(b, "Space", " ");
    await key(b, "ArrowLeft", "ArrowLeft");
    await key(b, "Escape", "Escape");
    expect(saved()).toHaveLength(0);
    expect(invokeMock.mock.calls.some(([command]) => command === "hide_launchpad")).toBe(false);
    expect(screen.queryByTestId("drop-slot")).toBeNull();
  });

  it("still opens a tile on Enter and on a click", async () => {
    await renderShown();
    fireEvent.keyDown(tile("c"), { code: "Enter", key: "Enter" });
    fireEvent.click(tile("a"));
    const launched = invokeMock.mock.calls.filter(([command]) => command === "launch_app");
    expect(launched.map(([, args]) => args)).toEqual([{ appId: "c" }, { appId: "a" }]);
  });

  it("rolls back and says so when saving fails", async () => {
    await renderShown();
    invokeMock.mockImplementation((command) =>
      command === "set_launchpad_layout"
        ? Promise.reject(new Error("disk full"))
        : Promise.resolve(null),
    );
    const a = tile("a");
    a.focus();
    await key(a, "Space", " ");
    await key(a, "ArrowRight", "ArrowRight");
    await key(a, "Space", " ");
    await flush();
    expect(saved()).toHaveLength(1);
    expect(ids(useAppStore.getState().state?.launchpad?.pages)).toEqual([
      ["a", "b", "hid", "c"],
      ["d"],
    ]);
    expect(screen.getByText("Couldn't save the new order.")).toBeTruthy();
  });

  it("does not drag in search results", async () => {
    await renderShown();
    fireEvent.keyDown(document.body, { key: "a" });
    expect(document.querySelector("[data-sortable]")).toBeNull();
    const result = tile("a");
    result.focus();
    await key(result, "Space", " ");
    expect(screen.queryByTestId("drop-slot")).toBeNull();
  });
});

import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, FlyoutShow } from "../../bindings";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { emitMock } from "../../test/tauriMock";
import { FlyoutRoot } from "./FlyoutRoot";

/** 20 apps, as in Preston's "IDE's" group: three 3 x 3 pages. */
const IDS = Array.from({ length: 20 }, (_, i) => `app${i}`);

const state = (settings: AppState["settings"] = {}): AppState => ({
  apps: IDS.map((id) => ({ id, name: id })),
  groups: [{ id: "ides", name: "IDE's", appIds: IDS }],
  settings,
});

const SHOW: FlyoutShow = {
  groupId: "ides",
  apps: IDS,
  size: { width: 232, height: 360 },
  origin: { edge: "bottom", x: 116, y: 360 },
  columns: 3,
  rows: 3,
  iconPx: 48,
};

async function renderShown(show: FlyoutShow = SHOW) {
  const view = render(<FlyoutRoot />);
  await act(() => Promise.resolve());
  act(() => emitMock("flyout:show", show));
  return view;
}

const track = (view: { container: HTMLElement }) =>
  view.container.querySelector<HTMLElement>(".launchpad-track");
const page = (view: { container: HTMLElement }) => track(view)?.dataset.page;
const tile = (name: string) => screen.getByRole("button", { name });
const active = () => document.activeElement?.getAttribute("aria-label");

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(state());
  vi.spyOn(console, "info").mockImplementation(() => undefined);
});

describe("flyout paging", () => {
  it("shows one page at a time with a dot per page", async () => {
    const view = await renderShown();
    expect(page(view)).toBe("0");
    const dots = screen.getAllByRole("button", { name: /^Page \d of 3$/ });
    expect(dots).toHaveLength(3);
    expect(dots[0]?.getAttribute("aria-current")).toBe("page");
    expect(screen.queryByRole("button", { name: "Next page" })).toBeNull();
  });

  it("goes on to the next page with Right past the edge, and back with Left", async () => {
    const view = await renderShown();
    fireEvent.keyDown(tile("app0"), { key: "ArrowRight" });
    fireEvent.keyDown(tile("app1"), { key: "ArrowRight" });
    expect(page(view)).toBe("0");
    fireEvent.keyDown(tile("app2"), { key: "ArrowRight" });
    expect(page(view)).toBe("1");
    expect(active()).toBe("app9");
    fireEvent.keyDown(tile("app9"), { key: "ArrowLeft" });
    expect(page(view)).toBe("0");
    expect(active()).toBe("app2");
  });

  it("turns with Page Down and with the arrows when no tile has focus", async () => {
    const view = await renderShown();
    tile("app4").focus();
    fireEvent.keyDown(tile("app4"), { key: "PageDown" });
    expect(page(view)).toBe("1");
    expect(active()).toBe("app13");
    (document.activeElement as HTMLElement).blur();
    fireEvent.keyDown(document.body, { key: "ArrowRight" });
    expect(page(view)).toBe("2");
    fireEvent.keyDown(document.body, { key: "ArrowRight" });
    expect(page(view)).toBe("2");
  });

  it("turns one page per wheel notch", async () => {
    const view = await renderShown();
    const pages = view.container.querySelector(".launchpad-pages");
    if (pages === null) throw new Error("no pages");
    fireEvent.wheel(pages, { deltaY: 100 });
    expect(page(view)).toBe("1");
    // The same gesture (events close together) turns no further.
    fireEvent.wheel(pages, { deltaY: 100 });
    expect(page(view)).toBe("1");
    // The next notch, after a pause, turns back.
    await act(() => new Promise((done) => setTimeout(done, 250)));
    fireEvent.wheel(pages, { deltaY: -100 });
    expect(page(view)).toBe("0");
  });

  it("jumps with a dot and steps with the optional arrows", async () => {
    useAppStore.getState().applyState(state({ showPageArrows: true }));
    const view = await renderShown();
    fireEvent.click(screen.getByRole("button", { name: "Page 3 of 3" }));
    expect(page(view)).toBe("2");
    expect(screen.getByRole("button", { name: "Next page" })).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByRole("button", { name: "Previous page" }));
    expect(page(view)).toBe("1");
  });

  it("starts every show on the first page", async () => {
    const view = await renderShown();
    fireEvent.click(screen.getByRole("button", { name: "Page 2 of 3" }));
    expect(page(view)).toBe("1");
    act(() => emitMock("flyout:show", SHOW));
    expect(page(view)).toBe("0");
    expect(active()).toBe("app0");
  });

  it("has no dots for a group that fits one page", async () => {
    await renderShown({ ...SHOW, apps: IDS.slice(0, 9) });
    expect(screen.queryByRole("navigation", { name: "Pages" })).toBeNull();
  });
});

import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, LaunchpadItem } from "../../bindings";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";

// 40 apps on one stored page: jsdom's 1024 x 768 window fits 35, so the
// overflow shows on a second page, and a third stored page follows.
const ids = Array.from({ length: 45 }, (_, i) => `app${String(i).padStart(2, "0")}`);
const item = (appId: string): LaunchpadItem => ({ kind: "app", appId });
const STATE: AppState = {
  apps: ids.map((id) => ({ id, name: `App ${id}` })),
  groups: [],
  launchpad: { pages: [ids.slice(0, 40).map(item), ids.slice(40).map(item)] },
};

const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

const flush = () => act(() => Promise.resolve());
const dots = () => screen.getAllByRole("button", { name: /^Page / });
const current = () => dots().findIndex((dot) => dot.getAttribute("aria-current") === "page");
const pages = () => screen.getByTestId("launchpad").querySelector(".launchpad-pages");
const track = () => screen.getByTestId("launchpad").querySelector<HTMLElement>(".launchpad-track");

async function renderShown() {
  render(<LaunchpadRoot />);
  await flush();
  act(() => emitMock("launchpad:show", SHOW));
}

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  invokeMock.mockResolvedValue(null);
});

describe("Launchpad paging", () => {
  it("flows a stored page's overflow onto its own page, one dot per page", async () => {
    await renderShown();
    expect(dots()).toHaveLength(3);
    expect(current()).toBe(0);
    const shown = screen.getByRole("group", { name: "Apps" }).querySelectorAll(".tile");
    expect(shown).toHaveLength(35);
    // The stored layout is left alone.
    expect(useAppStore.getState().state?.launchpad?.pages?.[0]).toHaveLength(40);
  });

  it("jumps to a page from its dot, sliding the track with transform", async () => {
    await renderShown();
    fireEvent.click(dots()[2] as HTMLElement);
    expect(current()).toBe(2);
    expect(track()?.style.transform).toBe("translate3d(calc(-200% + 0px), 0, 0)");
    expect(track()?.classList.contains("is-sliding")).toBe(true);
    expect(invokeMock.mock.calls.some(([c]) => c === "hide_launchpad")).toBe(false);
  });

  it("turns with the arrow and page keys, and only without a search", async () => {
    await renderShown();
    fireEvent.keyDown(document.body, { key: "ArrowRight" });
    expect(current()).toBe(1);
    fireEvent.keyDown(document.body, { key: "PageDown" });
    fireEvent.keyDown(document.body, { key: "PageDown" });
    expect(current()).toBe(2);
    fireEvent.keyDown(document.body, { key: "PageUp" });
    fireEvent.keyDown(document.body, { key: "ArrowLeft" });
    fireEvent.keyDown(document.body, { key: "ArrowLeft" });
    expect(current()).toBe(0);
  });

  it("turns one page per wheel gesture, either axis", async () => {
    await renderShown();
    const wheel = (deltaX: number, deltaY: number, timeStamp: number) => {
      const event = new WheelEvent("wheel", { deltaX, deltaY, bubbles: true });
      Object.defineProperty(event, "timeStamp", { value: timeStamp });
      fireEvent(pages() as Element, event);
    };
    wheel(0, 100, 1000);
    wheel(0, 100, 1050);
    expect(current()).toBe(1);
    wheel(80, 0, 2000);
    expect(current()).toBe(2);
    wheel(0, -100, 3000);
    expect(current()).toBe(1);
  });

  it("turns on a background drag past a quarter of the page, and swallows its click", async () => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1000);
    await renderShown();
    const area = pages() as Element;
    const background = area.querySelector(".launchpad-page") as Element;
    fireEvent.pointerDown(background, { pointerId: 1, button: 0, isPrimary: true, clientX: 600 });
    fireEvent.pointerMove(background, { pointerId: 1, isPrimary: true, clientX: 500, clientY: 0 });
    expect(track()?.classList.contains("is-sliding")).toBe(false);
    expect(track()?.style.transform).toBe("translate3d(calc(0% + -100px), 0, 0)");
    fireEvent.pointerUp(area, { pointerId: 1, isPrimary: true, clientX: 300 });
    fireEvent.click(background);
    expect(current()).toBe(1);
    expect(invokeMock.mock.calls.some(([c]) => c === "hide_launchpad")).toBe(false);
    vi.restoreAllMocks();
  });

  it("leaves a drag that starts on a tile to the tile", async () => {
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1000);
    await renderShown();
    const tile = screen.getByRole("button", { name: "App app00" });
    fireEvent.pointerDown(tile, { pointerId: 1, button: 0, isPrimary: true, clientX: 600 });
    fireEvent.pointerMove(tile, { pointerId: 1, isPrimary: true, clientX: 500, clientY: 0 });
    expect(track()?.style.transform).toBe("translate3d(calc(0% + 0px), 0, 0)");
    fireEvent.pointerUp(tile, { pointerId: 1, isPrimary: true, clientX: 300 });
    expect(current()).toBe(0);
    vi.restoreAllMocks();
  });

  it("goes back to page 1 on every show, without sliding", async () => {
    await renderShown();
    fireEvent.click(dots()[2] as HTMLElement);
    act(() => emitMock("launchpad:show", SHOW));
    expect(current()).toBe(0);
    expect(track()?.classList.contains("is-sliding")).toBe(false);
  });
});

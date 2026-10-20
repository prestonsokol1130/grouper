import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, FlyoutShow } from "../../bindings";
import { Toaster } from "../../components/Toaster";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { FlyoutRoot } from "./FlyoutRoot";

const STATE: AppState = {
  apps: ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"].map((name) => ({
    id: name.toLowerCase(),
    name,
  })),
  groups: [
    { id: "g1", name: "Work", appIds: ["alpha", "beta", "gamma", "delta", "epsilon"] },
    { id: "g2", name: "Empty", appIds: [] },
  ],
};

const payload = (groupId: string, apps: string[]): FlyoutShow => ({
  groupId,
  apps,
  size: { width: 232, height: 236 },
  origin: { edge: "bottom", x: 116, y: 236 },
  columns: 3,
  rows: 3,
  iconPx: 48,
});

const FIVE = payload("g1", ["alpha", "beta", "gamma", "delta", "epsilon"]);

const flush = () => act(() => Promise.resolve());

/** Renders the flyout and delivers `show` like the backend does. */
async function renderShown(show: FlyoutShow = FIVE) {
  const view = render(
    <>
      <FlyoutRoot />
      <Toaster />
    </>,
  );
  await flush();
  act(() => emitMock("flyout:show", show));
  return view;
}

const panel = (view: { container: HTMLElement }) =>
  view.container.querySelector<HTMLElement>(".flyout-panel");
const tile = (name: string) => screen.getByRole("button", { name });
const invoked = (command: string) => invokeMock.mock.calls.filter(([c]) => c === command);

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  vi.spyOn(console, "info").mockImplementation(() => undefined);
});

afterEach(() => vi.useRealTimers());

describe("FlyoutRoot", () => {
  it("shows the group name and a 3 x 3 page of its apps, focusing the first", async () => {
    const view = await renderShown();
    expect(screen.getByRole("heading").textContent).toBe("Work");
    const grid = screen.getByRole("group", { name: "Apps" });
    // 48px icons in 64 x 84 cells, three across and three down.
    expect(grid.style.gridTemplateColumns).toBe("repeat(3, 64px)");
    expect(grid.style.gridTemplateRows).toBe("repeat(3, 84px)");
    expect(grid.querySelectorAll(".tile")).toHaveLength(5);
    expect(document.activeElement).toBe(tile("Alpha"));
    expect(screen.queryByRole("navigation", { name: "Pages" })).toBeNull();
    expect(panel(view)?.classList.contains("is-opening")).toBe(true);
    expect(panel(view)?.dataset.motion).toBe("genie");
    expect(panel(view)?.style.getPropertyValue("--origin-x")).toBe("116px");
    expect(panel(view)?.style.getPropertyValue("--motion-duration")).toBe("300ms");
    expect(panel(view)?.style.getPropertyValue("--motion-close-duration")).toBe("180ms");
  });

  it("swaps to another group, layout and icon size on the next show", async () => {
    await renderShown();
    act(() =>
      emitMock("flyout:show", {
        ...payload("g1", ["beta", "gamma"]),
        columns: 4,
        rows: 2,
        iconPx: 64,
      }),
    );
    const grid = screen.getByRole("group", { name: "Apps" });
    expect(grid.style.gridTemplateColumns).toBe("repeat(4, 80px)");
    expect(grid.style.gridTemplateRows).toBe("repeat(2, 100px)");
    expect(document.activeElement).toBe(tile("Beta"));
  });

  it("hides the focus ring it opens with until a key is pressed", async () => {
    await renderShown();
    const root = document.documentElement;
    expect(document.activeElement).toBe(tile("Alpha"));
    expect(root.dataset.focusQuiet).toBe("");
    fireEvent.keyDown(tile("Alpha"), { key: "ArrowRight" });
    expect(root.dataset.focusQuiet).toBeUndefined();
    // Every show is quiet again.
    act(() => emitMock("flyout:show", FIVE));
    expect(root.dataset.focusQuiet).toBe("");
  });

  it("shows the focus ring on open with Always show keyboard focus", async () => {
    useAppStore.getState().applyState({ ...STATE, settings: { alwaysShowFocus: true } });
    await renderShown();
    const root = document.documentElement;
    expect(root.dataset.focusQuiet).toBeUndefined();
    expect(root.dataset.focusAlways).toBe("");
    expect(document.activeElement).toBe(tile("Alpha"));
    useAppStore.getState().applyState(STATE);
    act(() => emitMock("flyout:show", FIVE));
    expect(root.dataset.focusAlways).toBeUndefined();
  });

  it("moves focus with the arrow keys and launches with Enter", async () => {
    invokeMock.mockResolvedValue(null);
    await renderShown();
    fireEvent.keyDown(tile("Alpha"), { key: "ArrowRight" });
    expect(document.activeElement).toBe(tile("Beta"));
    fireEvent.keyDown(tile("Beta"), { key: "ArrowDown" });
    expect(document.activeElement).toBe(tile("Epsilon"));
    fireEvent.keyDown(tile("Epsilon"), { key: "ArrowUp" });
    expect(document.activeElement).toBe(tile("Beta"));
    fireEvent.keyDown(tile("Beta"), { key: "Enter" });
    await flush();
    expect(invoked("launch_from_flyout")).toEqual([["launch_from_flyout", { appId: "beta" }]]);
  });

  it("keeps the flyout open with a friendly toast when a launch fails", async () => {
    await renderShown();
    invokeMock.mockRejectedValueOnce("App not found: Gamma");
    fireEvent.click(tile("Gamma"));
    await flush();
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Couldn't open Gamma.");
    expect(alert.textContent).toContain("It may have been moved or uninstalled.");
    expect(alert.textContent).not.toContain("App not found");
    expect(invoked("hide_flyout")).toHaveLength(0);
  });

  it("plays the close animation on Escape, then hides", async () => {
    vi.useFakeTimers();
    const view = await renderShown();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(panel(view)?.classList.contains("is-closing")).toBe(true);
    expect(invoked("hide_flyout")).toHaveLength(0);
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_flyout")).toHaveLength(1);
  });

  it("opens back up when the hide fails, and Escape works again", async () => {
    vi.useFakeTimers();
    const view = await renderShown();
    // What Tauri rejects with when hide_flyout returns the Rust Err.
    invokeMock.mockRejectedValueOnce("could not hide the flyout: ShowWindow failed");
    fireEvent.keyDown(window, { key: "Escape" });
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_flyout")).toHaveLength(1);
    expect(panel(view)?.classList.contains("is-closing")).toBe(false);
    expect(panel(view)?.classList.contains("is-opening")).toBe(true);
    invokeMock.mockResolvedValue(null);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(panel(view)?.classList.contains("is-closing")).toBe(true);
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_flyout")).toHaveLength(2);
  });

  it("does not hide when a new show arrives during the close", async () => {
    vi.useFakeTimers();
    await renderShown();
    fireEvent.keyDown(window, { key: "Escape" });
    act(() => emitMock("flyout:show", FIVE));
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_flyout")).toHaveLength(0);
  });

  it("hides its content when the window is hidden so the next show starts fresh", async () => {
    const view = await renderShown();
    act(() => emitMock("flyout:hide", null));
    expect(panel(view)?.classList.contains("is-hidden")).toBe(true);
  });

  it("uses the 120ms fade when Windows animation effects are off", async () => {
    const real = window.matchMedia;
    window.matchMedia = (query: string) =>
      ({ ...real(query), matches: query.includes("reduced-motion") }) as MediaQueryList;
    const view = await renderShown();
    expect(panel(view)?.dataset.motion).toBe("reduced");
    expect(panel(view)?.style.getPropertyValue("--motion-duration")).toBe("120ms");
    window.matchMedia = real;
  });

  it("says what to do for an empty group", async () => {
    await renderShown(payload("g2", []));
    expect(screen.getByRole("heading").textContent).toBe("Empty");
    expect(
      screen.getByText("No apps in this group yet. Drag apps here or click Add apps."),
    ).toBeTruthy();
  });

  it("catches up with the last show when the page loads late", async () => {
    invokeMock.mockImplementation((command) =>
      Promise.resolve(command === "get_flyout" ? FIVE : null),
    );
    render(<FlyoutRoot />);
    await flush();
    expect(screen.getByRole("heading").textContent).toBe("Work");
  });

  it("puts a 50-app group on six 3 x 3 pages that never scroll", async () => {
    const ids = Array.from({ length: 50 }, (_, i) => `app${i}`);
    useAppStore.getState().applyState({
      apps: ids.map((id) => ({ id, name: `${id} ${"long name ".repeat(20)}` })),
      groups: [{ id: "big", name: "Big", appIds: ids }],
    });
    const view = await renderShown(payload("big", ids));
    expect(screen.getAllByRole("button", { name: /^Page \d of 6$/ })).toHaveLength(6);
    expect(panel(view)?.className).toContain("overflow-hidden");
  });
});

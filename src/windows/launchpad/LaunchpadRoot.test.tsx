import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState } from "../../bindings";
import { Toaster } from "../../components/Toaster";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";

const STATE: AppState = {
  apps: [
    { id: "alpha", name: "Alpha" },
    { id: "beta", name: "Beta" },
    { id: "secret", name: "Secret", hidden: true },
  ],
  groups: [{ id: "g1", name: "Work", appIds: ["alpha", "beta"] }],
  launchpad: {
    pages: [
      [
        { kind: "app", appId: "alpha" },
        { kind: "folder", groupId: "g1" },
        { kind: "app", appId: "secret" },
        { kind: "app", appId: "beta" },
      ],
      [{ kind: "app", appId: "page-two" }],
    ],
  },
};

const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

const flush = () => act(() => Promise.resolve());
const invoked = (command: string) => invokeMock.mock.calls.filter(([c]) => c === command);
const tile = (name: string) => screen.getByRole("button", { name });
const root = () => screen.getByTestId("launchpad");

async function renderShown(show: LaunchpadShow = SHOW) {
  render(
    <>
      <LaunchpadRoot />
      <Toaster />
    </>,
  );
  await flush();
  act(() => emitMock("launchpad:show", show));
}

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  invokeMock.mockResolvedValue(null);
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

afterEach(() => vi.useRealTimers());

describe("LaunchpadRoot", () => {
  it("shows page 1 as a 7 x 5 grid of app and folder tiles, without hidden apps", async () => {
    await renderShown();
    const grid = screen.getByRole("group", { name: "Apps" });
    expect(grid.style.gridTemplateColumns).toBe("repeat(7, minmax(0, 1fr))");
    expect(grid.style.gridTemplateRows).toBe("repeat(5, minmax(0, 1fr))");
    const names = [...grid.querySelectorAll(".tile")].map((t) => t.getAttribute("aria-label"));
    expect(names).toEqual(["Alpha", "Work", "Beta"]);
    expect(grid.querySelector('[data-testid="folder-art"]')).not.toBeNull();
  });

  it("plays the open animation from the origin Rust sends", async () => {
    await renderShown({ origin: { edge: "top", x: 0, y: 0 } });
    expect(root().classList.contains("is-opening")).toBe(true);
    expect(root().dataset.motion).toBe("genie");
    expect(root().dataset.edge).toBe("top");
    expect(root().style.getPropertyValue("--origin-x")).toBe("0px");
  });

  it("hides when the background is clicked, after the close animation", async () => {
    vi.useFakeTimers();
    await renderShown();
    fireEvent.click(screen.getByRole("group", { name: "Apps" }));
    expect(root().classList.contains("is-closing")).toBe(true);
    expect(invoked("hide_launchpad")).toHaveLength(0);
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(1);
    expect(invoked("launch_app")).toHaveLength(0);
  });

  it("launches a clicked app and then hides, but a tile click alone never hides", async () => {
    vi.useFakeTimers();
    await renderShown();
    fireEvent.click(tile("Alpha"));
    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(invoked("launch_app")).toEqual([["launch_app", { appId: "alpha" }]]);
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(1);
    fireEvent.click(tile("Work"));
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(1);
  });

  it("stays open with a friendly toast when a launch fails", async () => {
    await renderShown();
    invokeMock.mockRejectedValueOnce("App not found: Beta");
    fireEvent.click(tile("Beta"));
    await flush();
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Couldn't open Beta.");
    expect(alert.textContent).not.toContain("App not found");
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(invoked("hide_launchpad")).toHaveLength(0);
  });

  it("hides on Escape", async () => {
    vi.useFakeTimers();
    await renderShown();
    fireEvent.keyDown(window, { key: "Escape" });
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(1);
  });

  it("does not hide when a new show arrives during the close", async () => {
    vi.useFakeTimers();
    await renderShown();
    fireEvent.keyDown(window, { key: "Escape" });
    act(() => emitMock("launchpad:show", SHOW));
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(0);
  });

  it("opens back up when the hide fails", async () => {
    vi.useFakeTimers();
    await renderShown();
    invokeMock.mockRejectedValueOnce("could not hide the Launchpad");
    fireEvent.keyDown(window, { key: "Escape" });
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(root().classList.contains("is-opening")).toBe(true);
  });

  it("hides its content when the window is hidden, and uses the fade with reduced motion", async () => {
    const real = window.matchMedia;
    window.matchMedia = (query: string) =>
      ({ ...real(query), matches: query.includes("reduced-motion") }) as MediaQueryList;
    await renderShown();
    expect(root().dataset.motion).toBe("reduced");
    expect(root().style.getPropertyValue("--motion-duration")).toBe("120ms");
    act(() => emitMock("launchpad:hide", null));
    expect(root().classList.contains("is-hidden")).toBe(true);
    window.matchMedia = real;
  });

  it("says so when there are no apps yet", async () => {
    useAppStore.getState().applyState({ ...STATE, launchpad: { pages: [] } });
    await renderShown();
    expect(screen.getByText("No apps yet")).toBeTruthy();
  });
});

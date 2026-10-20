import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState, LaunchpadItem } from "../../bindings";
import { Toaster } from "../../components/Toaster";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";

const app = (appId: string): LaunchpadItem => ({ kind: "app", appId });
const STATE: AppState = {
  apps: ["a", "b", "x", "y"].map((id) => ({ id, name: `App ${id}` })),
  groups: [{ id: "g", name: "Tools", appIds: ["x", "y"] }],
  launchpad: { pages: [[app("a"), app("b"), { kind: "folder", groupId: "g" }]] },
};
const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

/** What each command answers; tests override some. */
let replies: Record<string, unknown> = {};
const calls = (name: string) => invokeMock.mock.calls.filter(([command]) => command === name);
const flush = () => act(() => Promise.resolve());
/** Lets a close animation finish (it falls back to a timer in jsdom). */
const settle = async () => {
  act(() => vi.advanceTimersByTime(600));
  await flush();
};
const dialog = () => screen.queryByRole("dialog");
const escape = () =>
  act(() => fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" }));

async function openFolder() {
  act(() => fireEvent.click(screen.getByRole("button", { name: "Tools" })));
  await flush();
  const panel = dialog();
  if (panel === null) throw new Error("the folder did not open ");
  return within(panel);
}

beforeEach(async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  replies = {
    get_state: STATE,
    launch_group: { launched: 2, failed: [] },
    create_group_shortcut: "C:\\g.lnk",
  };
  invokeMock.mockImplementation((command) => Promise.resolve(replies[command] ?? null));
  vi.spyOn(console, "error").mockImplementation(() => undefined);
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

describe("Launchpad folder", () => {
  it("opens a folder tile as a dialog with its apps", async () => {
    const panel = await openFolder();
    expect(dialog()?.getAttribute("aria-labelledby")).toBeTruthy();
    expect(panel.getByRole("button", { name: "App x" })).toBeTruthy();
    expect(panel.getByRole("button", { name: "App y" })).toBeTruthy();
    expect(
      screen.getByTestId("launchpad").querySelector(".launchpad-main")?.hasAttribute("inert"),
    ).toBe(true);
  });

  it("renames inline: Enter saves, Escape cancels, an empty name is refused", async () => {
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "Tools" })));
    let box = panel.getByRole("textbox", { name: "Group name" });
    act(() => fireEvent.change(box, { target: { value: "  " } }));
    act(() => fireEvent.keyDown(box, { key: "Enter" }));
    expect(panel.getByRole("alert").textContent).toBe("Give the group a name.");
    act(() => fireEvent.change(box, { target: { value: "Work" } }));
    act(() => fireEvent.keyDown(box, { key: "Enter" }));
    await flush();
    expect(calls("rename_group").map(([, args]) => args)).toEqual([{ id: "g", name: "Work" }]);
    expect(panel.getByRole("button", { name: "Work" })).toBeTruthy();

    act(() => fireEvent.click(panel.getByRole("button", { name: "Work" })));
    box = panel.getByRole("textbox", { name: "Group name" });
    act(() => fireEvent.change(box, { target: { value: "Other" } }));
    act(() => fireEvent.keyDown(box, { key: "Escape" }));
    await settle();
    expect(calls("rename_group")).toHaveLength(1);
    // Escape in the box only cancels the edit; the folder stays open.
    expect(dialog()).not.toBeNull();
  });

  it("closes the folder on the first Escape and the Launchpad on the second", async () => {
    await openFolder();
    escape();
    await settle();
    expect(dialog()).toBeNull();
    expect(calls("hide_launchpad")).toHaveLength(0);
    escape();
    await settle();
    expect(calls("hide_launchpad")).toHaveLength(1);
  });

  it("closes only the folder on a click outside it", async () => {
    await openFolder();
    const scrim = dialog()?.parentElement;
    if (!scrim) throw new Error("no scrim");
    act(() => fireEvent.click(scrim));
    await settle();
    expect(dialog()).toBeNull();
    expect(calls("hide_launchpad")).toHaveLength(0);
  });

  it("launches an app from the folder", async () => {
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "App x" })));
    await flush();
    expect(calls("launch_app").map(([, args]) => args)).toEqual([{ appId: "x" }]);
  });

  it("Launch All hides the Launchpad when every app opened", async () => {
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "Launch All" })));
    await flush();
    await settle();
    expect(calls("launch_group").map(([, args]) => args)).toEqual([{ groupId: "g" }]);
    expect(calls("hide_launchpad")).toHaveLength(1);
  });

  it("Launch All names the apps that failed and keeps the Launchpad open", async () => {
    replies.launch_group = { launched: 1, failed: ["Spotify", "Slack"] };
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "Launch All" })));
    await flush();
    await settle();
    expect(screen.getByText("Couldn't open Spotify and Slack.")).toBeTruthy();
    expect(screen.getByText("They may have been moved or uninstalled.")).toBeTruthy();
    expect(calls("hide_launchpad")).toHaveLength(0);
  });

  it("pins: creates the shortcut, shows the guide, then reveals it", async () => {
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "Pin Group to Windows Taskbar" })));
    await flush();
    expect(calls("create_group_shortcut").map(([, args]) => args)).toEqual([{ groupId: "g" }]);
    expect(panel.getByTestId("pin-guide").textContent).toContain("Choose Pin to taskbar.");
    expect(calls("hide_launchpad")).toHaveLength(0);
    act(() => fireEvent.click(panel.getByRole("button", { name: "Show in Explorer" })));
    await flush();
    await settle();
    expect(calls("reveal_group_shortcut").map(([, args]) => args)).toEqual([{ groupId: "g" }]);
    expect(calls("hide_launchpad")).toHaveLength(1);
  });

  it("drags an app out past the panel's edge onto the grid", async () => {
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (
      this: Element,
    ) {
      const inPanel = this.getAttribute("role") === "dialog";
      return inPanel
        ? DOMRect.fromRect({ x: 500, y: 300, width: 400, height: 300 })
        : DOMRect.fromRect({ x: 0, y: 0, width: 0, height: 0 });
    });
    const panel = await openFolder();
    const tile = panel.getByRole("button", { name: "App y" });
    const at = (x: number, y: number) => ({ clientX: x, clientY: y, button: 0, isPrimary: true });
    act(() => fireEvent.pointerDown(tile, at(600, 400)));
    act(() => fireEvent.pointerMove(document, at(620, 400)));
    await flush();
    act(() => fireEvent.pointerMove(document, at(10, 10)));
    await flush();
    expect(dialog()?.parentElement?.hasAttribute("data-out")).toBe(true);
    act(() => fireEvent.pointerUp(document, at(10, 10)));
    await flush();
    // dnd-kit stops swallowing clicks 50ms after the drop.
    act(() => vi.advanceTimersByTime(60));
    expect(calls("remove_app_from_folder").map(([, args]) => args)).toEqual([
      { groupId: "g", appId: "y", to: { page: 0, index: 0 } },
    ]);
    // One app left: the folder dissolved, so the panel is gone.
    expect(dialog()).toBeNull();
    expect(useAppStore.getState().state?.groups).toEqual([]);
  });

  it("reads Show taskbar shortcut once the shortcut exists", async () => {
    useAppStore.getState().applyState({
      ...STATE,
      groups: [{ id: "g", name: "Tools", appIds: ["x", "y"], hasShortcut: true }],
    });
    const panel = await openFolder();
    act(() => fireEvent.click(panel.getByRole("button", { name: "Show taskbar shortcut" })));
    await flush();
    expect(calls("create_group_shortcut")).toHaveLength(0);
    expect(panel.getByTestId("pin-guide")).toBeTruthy();
  });
});

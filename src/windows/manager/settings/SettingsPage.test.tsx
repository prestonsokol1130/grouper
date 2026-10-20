import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { AppState } from "../../../bindings";
import { useAppStore } from "../../../store/appStore";
import { markHotkeyFailed, useHotkeyStatus } from "../../../store/hotkeyStatus";
import { invokeMock } from "../../../test/tauriMock";
import { SettingsPage } from "./SettingsPage";

const STATE: AppState = { apps: [], groups: [], settings: { firstRunDone: true } };

const calls = (command: string) =>
  invokeMock.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

/** Answers each command; `set_hotkey` fails when `takenHotkey` is asked for. */
function backend(takenHotkey = "") {
  invokeMock.mockImplementation((command, args) => {
    const hotkey = (args as { hotkey?: string } | undefined)?.hotkey;
    if (command === "set_hotkey" && hotkey === takenHotkey) return Promise.reject("in use");
    if (command === "rescan_apps") return Promise.resolve(42);
    if (command === "plugin:app|version") return Promise.resolve("0.1.0");
    return Promise.resolve(STATE.settings);
  });
}

const card = (title: string) => {
  const heading = screen.getByText(title);
  const element = heading.closest(".setting-card");
  if (!(element instanceof HTMLElement)) throw new Error(`no card ${title}`);
  return element;
};

beforeEach(() => {
  useAppStore.getState().applyState(STATE);
  useHotkeyStatus.setState({ failed: null });
  backend();
});

describe("settings page", () => {
  it("shows every DESIGN.md setting with its default and no theme setting", async () => {
    render(<SettingsPage />);
    await act(async () => {});
    for (const title of [
      "Launchpad shortcut",
      "Hot corner: top left",
      "Hot corner: top right",
      "Accent color",
      "Window background",
      "Folder shape",
      "Launchpad icon size",
      "Flyout layout",
      "Flyout icon size",
      "Show page arrows",
      "Always show keyboard focus",
      "Launchpad grid",
      "Animation style",
      "Start with Windows",
      "Rescan Start Menu",
      "Show welcome again",
      "Logs",
      "Data",
    ]) {
      expect(screen.getByText(title)).toBeTruthy();
    }
    expect(screen.queryByText(/theme/i)).toBeNull();
    expect(screen.getByRole("combobox", { name: "Folder shape" })).toHaveProperty(
      "value",
      "square",
    );
    expect(screen.getByRole("combobox", { name: "Columns" })).toHaveProperty("value", "7");
    expect(screen.getByText("Version 0.1.0")).toBeTruthy();
  });

  it("saves a changed setting through update_settings", () => {
    render(<SettingsPage />);
    const pick = (name: string, value: string) =>
      fireEvent.change(screen.getByRole("combobox", { name }), { target: { value } });
    pick("Launchpad icon size", "large");
    pick("Rows", "6");
    pick("Animation style", "smooth");
    pick("Window background", "solid");
    pick("Folder shape", "squircle");
    pick("Accent color", "custom");
    expect(calls("update_settings")).toEqual([
      { patch: { iconSize: "large" } },
      { patch: { gridRows: 6 } },
      { patch: { animation: "smooth" } },
      { patch: { background: "solid" } },
      { patch: { folderShape: "squircle" } },
      { patch: { accent: "#0067C0" } },
    ]);
  });

  it("saves the flyout and accessibility settings, defaults first", () => {
    render(<SettingsPage />);
    const select = (name: string) => screen.getByRole("combobox", { name });
    expect(select("Flyout layout")).toHaveProperty("value", "3x3");
    expect(select("Flyout icon size")).toHaveProperty("value", "matchTaskbar");
    const arrows = screen.getByRole("switch", { name: "Show page arrows" });
    const focus = screen.getByRole("switch", { name: "Always show keyboard focus" });
    expect(arrows).toHaveProperty("checked", false);
    expect(focus).toHaveProperty("checked", false);
    fireEvent.change(select("Flyout layout"), { target: { value: "4x2" } });
    fireEvent.change(select("Flyout icon size"), { target: { value: "large" } });
    fireEvent.click(arrows);
    fireEvent.click(focus);
    expect(calls("update_settings")).toEqual([
      { patch: { flyoutLayout: "4x2" } },
      { patch: { flyoutIconSize: "large" } },
      { patch: { showPageArrows: true } },
      { patch: { alwaysShowFocus: true } },
    ]);
  });

  it("turns a hot corner off and keeps the other", () => {
    render(<SettingsPage />);
    fireEvent.click(screen.getByRole("switch", { name: "Hot corner: top left" }));
    expect(calls("update_settings")).toEqual([
      { patch: { hotCorners: { topLeft: false, topRight: true } } },
    ]);
  });

  it("turns Start with Windows on through set_autostart", () => {
    render(<SettingsPage />);
    fireEvent.click(screen.getByRole("switch", { name: "Start with Windows" }));
    expect(calls("set_autostart")).toEqual([{ enabled: true }]);
  });

  it("rescans and shows the count, and opens the folders", async () => {
    render(<SettingsPage />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Rescan now/ }));
    });
    expect(within(card("Rescan Start Menu")).getByText("Found 42 apps.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Open logs folder/ }));
    fireEvent.click(screen.getByRole("button", { name: /Open data folder/ }));
    fireEvent.click(screen.getByRole("button", { name: "View on GitHub" }));
    expect(calls("open_place")).toEqual([{ place: "logs" }, { place: "data" }, { place: "repo" }]);
  });

  it("shows the welcome again", () => {
    render(<SettingsPage />);
    fireEvent.click(screen.getByRole("button", { name: "Show welcome" }));
    expect(calls("update_settings")).toEqual([{ patch: { firstRunDone: false } }]);
  });
});

describe("shortcut recorder", () => {
  const record = async (code: string, mods: Record<string, boolean>) => {
    fireEvent.click(within(card("Launchpad shortcut")).getByRole("button", { name: "Change" }));
    const box = screen.getByRole("textbox", { name: "Press the new shortcut" });
    await act(async () => {
      fireEvent.keyDown(box, { code, ...mods });
    });
  };

  it("shows the shortcut as chips", () => {
    render(<SettingsPage />);
    expect(screen.getByRole("img", { name: "Shortcut: Alt + `" })).toBeTruthy();
  });

  it("saves an allowed shortcut with set_hotkey", async () => {
    render(<SettingsPage />);
    await record("F12", { shiftKey: true, altKey: true });
    expect(calls("set_hotkey")).toEqual([{ hotkey: "Shift+Alt+F12" }]);
  });

  it("refuses a shortcut without a modifier or one Windows uses", async () => {
    render(<SettingsPage />);
    await record("KeyA", {});
    expect(screen.getByRole("alert").textContent).toBe("Use Ctrl, Alt or Win with another key.");
    const box = screen.getByRole("textbox", { name: "Press the new shortcut" });
    fireEvent.keyDown(box, { code: "KeyL", metaKey: true });
    expect(screen.getByRole("alert").textContent).toMatch(/Windows uses that shortcut/);
    expect(calls("set_hotkey")).toEqual([]);
    fireEvent.keyDown(box, { key: "Escape", code: "Escape" });
    expect(screen.queryByRole("textbox", { name: "Press the new shortcut" })).toBeNull();
  });

  it("keeps the old shortcut when another app holds the new one", async () => {
    backend("Ctrl+Alt+K");
    render(<SettingsPage />);
    await record("KeyK", { ctrlKey: true, altKey: true });
    expect(screen.getByRole("alert").textContent).toBe(
      "That shortcut is in use by another app. Pick a different one.",
    );
    expect(screen.getByRole("img", { name: "Shortcut: Alt + `" })).toBeTruthy();
  });

  it("shows the startup failure banner until a new shortcut works", async () => {
    render(<SettingsPage />);
    act(() => markHotkeyFailed({ hotkey: "Alt+`", error: "taken" }));
    expect(screen.getByRole("alert").textContent).toContain(
      "Alt + ` is already used by another app. Pick a different shortcut in Settings.",
    );
    await record("F12", { shiftKey: true, altKey: true });
    expect(screen.queryByRole("alert")).toBeNull();
  });
});

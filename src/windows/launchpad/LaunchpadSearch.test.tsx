import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState } from "../../bindings";
import type { LaunchpadShow } from "../../lib/events";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { emitMock, invokeMock } from "../../test/tauriMock";
import { LaunchpadRoot } from "./LaunchpadRoot";

const STATE: AppState = {
  apps: [
    { id: "vsc", name: "Visual Studio Code" },
    { id: "notepad", name: "Notepad" },
    { id: "notes", name: "Sticky Notes" },
    { id: "cafe", name: "Café Menu" },
    { id: "git", name: "Git Bash" },
    { id: "secret", name: "Notepad Secret", hidden: true },
  ],
  groups: [{ id: "dev", name: "Dev", appIds: ["git"] }],
  launchpad: {
    pages: [
      [
        { kind: "app", appId: "notes" },
        { kind: "app", appId: "vsc" },
        { kind: "app", appId: "notepad" },
        { kind: "app", appId: "secret" },
        { kind: "folder", groupId: "dev" },
      ],
      [{ kind: "app", appId: "cafe" }],
    ],
  },
};

const SHOW: LaunchpadShow = { origin: { edge: "bottom", x: 960, y: 1080 } };

const flush = () => act(() => Promise.resolve());
const invoked = (command: string) => invokeMock.mock.calls.filter(([c]) => c === command);
const search = () => screen.getByRole<HTMLInputElement>("searchbox", { name: "Search apps" });
const resultNames = () =>
  [...screen.getByRole("group", { name: "Search results" }).querySelectorAll(".tile")].map((t) =>
    t.getAttribute("aria-label"),
  );
const type = (value: string) => fireEvent.change(search(), { target: { value } });

async function renderShown() {
  render(<LaunchpadRoot />);
  await flush();
  act(() => emitMock("launchpad:show", SHOW));
}

beforeEach(() => {
  setPreviewIconUrls(new Map([["_fallback", "fallback.png"]]));
  useAppStore.getState().applyState(STATE);
  invokeMock.mockResolvedValue(null);
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

afterEach(() => vi.useRealTimers());

describe("Launchpad search", () => {
  it("focuses and selects the search on every show, and clears it on hide", async () => {
    await renderShown();
    expect(document.activeElement).toBe(search());
    type("note");
    search().blur();
    act(() => emitMock("launchpad:show", SHOW));
    expect(document.activeElement).toBe(search());
    expect(search().selectionStart).toBe(0);
    expect(search().selectionEnd).toBe(4);
    act(() => emitMock("launchpad:hide", null));
    expect(search().value).toBe("");
  });

  it("replaces the pages with ranked results on every keystroke", async () => {
    await renderShown();
    type("n");
    expect(resultNames()).toEqual(["Notepad", "Sticky Notes", "Café Menu"]);
    type("note");
    expect(resultNames()).toEqual(["Notepad", "Sticky Notes"]);
    expect(screen.queryByRole("navigation", { name: "Pages" })?.childElementCount ?? 0).toBe(0);
    type("");
    expect(screen.getByRole("group", { name: "Apps" })).toBeTruthy();
  });

  it("finds initials, ignores accents and shows a folder for an app inside it", async () => {
    await renderShown();
    type("vsc");
    expect(resultNames()).toEqual(["Visual Studio Code"]);
    type("CAFE");
    expect(resultNames()).toEqual(["Café Menu"]);
    type("git");
    expect(resultNames()).toEqual(["Dev"]);
  });

  it("sends typing anywhere into the search box", async () => {
    await renderShown();
    search().blur();
    fireEvent.keyDown(document.body, { key: "v" });
    fireEvent.keyDown(document.body, { key: "s" });
    expect(search().value).toBe("vs");
    expect(document.activeElement).toBe(search());
    search().blur();
    fireEvent.keyDown(document.body, { key: "Backspace" });
    expect(search().value).toBe("v");
    fireEvent.keyDown(document.body, { key: "c", ctrlKey: true });
    expect(search().value).toBe("v");
  });

  it("launches the first result on Enter", async () => {
    vi.useFakeTimers();
    await renderShown();
    type("notep");
    fireEvent.keyDown(search(), { key: "Enter" });
    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(invoked("launch_app")).toEqual([["launch_app", { appId: "notepad" }]]);
  });

  it("moves focus through the results with the arrow keys", async () => {
    await renderShown();
    type("n");
    fireEvent.keyDown(search(), { key: "ArrowDown" });
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Notepad");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "ArrowRight" });
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Sticky Notes");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "ArrowUp" });
    expect(document.activeElement).toBe(search());
  });

  it("clears the search on Escape first, and hides only when it is empty", async () => {
    vi.useFakeTimers();
    await renderShown();
    type("note");
    fireEvent.keyDown(search(), { key: "Escape" });
    expect(search().value).toBe("");
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(0);
    fireEvent.keyDown(search(), { key: "Escape" });
    await act(() => vi.advanceTimersByTimeAsync(600));
    expect(invoked("hide_launchpad")).toHaveLength(1);
  });

  it("says so when nothing matches", async () => {
    await renderShown();
    type("zzz");
    expect(screen.getByRole("status").textContent).toBe("No apps match “zzz”.");
    fireEvent.click(search());
    expect(invoked("hide_launchpad")).toHaveLength(0);
  });
});

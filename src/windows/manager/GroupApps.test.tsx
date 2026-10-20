import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { AppState } from "../../bindings";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { invokeMock } from "../../test/tauriMock";
import { ManagerRoot } from "./ManagerRoot";

const STATE: AppState = {
  // Past the first-run welcome, which otherwise replaces the Manager.
  settings: { firstRunDone: true },
  apps: [
    { id: "vsc", name: "Visual Studio Code" },
    { id: "term", name: "Terminal" },
    { id: "np", name: "Notepad" },
    { id: "git", name: "GitHub Desktop" },
    { id: "hid", name: "Hidden Tool", hidden: true },
  ],
  groups: [{ id: "g1", name: "Coding", appIds: ["term", "hid", "np"] }],
};

const calls = (command: string) =>
  invokeMock.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

const grid = () => screen.getByRole("list", { name: "Apps in Coding" });
const tileNames = () =>
  within(grid())
    .getAllByTestId("tile-label")
    .map((label) => label.textContent);

const openPicker = () => {
  fireEvent.click(screen.getByRole("button", { name: "Add apps" }));
  return screen.getByRole("dialog", { name: "Add apps to Coding" });
};

const box = (dialog: HTMLElement, name: string) =>
  within(dialog).getByRole("checkbox", { name }) as HTMLInputElement;

beforeEach(() => {
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  invokeMock.mockResolvedValue(null);
});

describe("group apps", () => {
  it("shows the group's visible apps in order", () => {
    render(<ManagerRoot />);
    expect(tileNames()).toEqual(["Terminal", "Notepad"]);
    expect(screen.getByText("2 apps")).toBeTruthy();
  });

  it("removes an app with its Remove button or Delete, keeping hidden ones", () => {
    render(<ManagerRoot />);
    fireEvent.click(screen.getByRole("button", { name: "Remove Terminal" }));
    expect(calls("set_group_apps")).toEqual([{ id: "g1", appIds: ["hid", "np"] }]);
    expect(tileNames()).toEqual(["Notepad"]);
    fireEvent.keyDown(screen.getByRole("button", { name: "Notepad" }), { key: "Delete" });
    expect(calls("set_group_apps")[1]).toEqual({ id: "g1", appIds: ["hid"] });
    expect(screen.getByText(/No apps in this group yet\. Drag apps here or click Add apps\./));
  });

  it("reorders with Alt+arrow keys", () => {
    render(<ManagerRoot />);
    fireEvent.keyDown(screen.getByRole("button", { name: "Terminal" }), {
      key: "ArrowRight",
      altKey: true,
    });
    expect(calls("set_group_apps")).toEqual([{ id: "g1", appIds: ["np", "hid", "term"] }]);
    expect(tileNames()).toEqual(["Notepad", "Terminal"]);
  });

  it("reorders by dragging a tile onto another slot", () => {
    render(<ManagerRoot />);
    const cells = [...grid().children] as HTMLElement[];
    cells.forEach((cell, index) => {
      cell.getBoundingClientRect = () =>
        ({ left: index * 96, top: 0, width: 88, height: 104 }) as DOMRect;
    });
    const first = cells[0] as HTMLElement;
    fireEvent.pointerDown(first, { button: 0, clientX: 40, clientY: 40, pointerId: 1 });
    fireEvent.pointerMove(first, { clientX: 140, clientY: 40, pointerId: 1 });
    expect(first.getAttribute("data-dragging")).toBe("true");
    expect((cells[1] as HTMLElement).style.translate).toBe("-96px 0px");
    fireEvent.pointerUp(first, { clientX: 140, clientY: 40, pointerId: 1 });
    expect(calls("set_group_apps")).toEqual([{ id: "g1", appIds: ["np", "hid", "term"] }]);
  });

  it("opens an app on click and says so when it can't", async () => {
    invokeMock.mockRejectedValue("App not found: Notepad");
    render(<ManagerRoot />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Notepad" }));
    });
    expect(calls("launch_app")).toEqual([{ appId: "np" }]);
    const toast = useToastStore.getState().toasts[0];
    expect(toast?.title).toBe("Couldn't open Notepad.");
    expect(toast?.message).toBe("It may have been moved or uninstalled.");
  });
});

describe("app picker", () => {
  it("lists visible apps sorted, with the group's apps ticked", () => {
    render(<ManagerRoot />);
    const dialog = openPicker();
    const names = within(dialog)
      .getAllByRole("checkbox")
      .map((input) => input.closest("label")?.getAttribute("title"));
    expect(names).toEqual(["GitHub Desktop", "Notepad", "Terminal", "Visual Studio Code"]);
    expect(box(dialog, "Terminal").checked).toBe(true);
    expect(box(dialog, "Notepad").checked).toBe(true);
    expect(box(dialog, "Visual Studio Code").checked).toBe(false);
    expect(document.activeElement).toBe(within(dialog).getByRole("searchbox"));
  });

  it("finds apps by substring and by initials", () => {
    render(<ManagerRoot />);
    const dialog = openPicker();
    const search = within(dialog).getByRole("searchbox");
    fireEvent.change(search, { target: { value: "vsc" } });
    expect(within(dialog).getAllByRole("checkbox")).toHaveLength(1);
    expect(box(dialog, "Visual Studio Code")).toBeTruthy();
    fireEvent.change(search, { target: { value: "DESK" } });
    expect(within(dialog).getAllByRole("checkbox")).toHaveLength(1);
    expect(box(dialog, "GitHub Desktop")).toBeTruthy();
    fireEvent.change(search, { target: { value: "zzz" } });
    expect(within(dialog).queryAllByRole("checkbox")).toHaveLength(0);
    expect(within(dialog).getByText('No apps match "zzz". Try another name.')).toBeTruthy();
    fireEvent.keyDown(search, { key: "Escape" });
    expect((search as HTMLInputElement).value).toBe("");
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("saves the ticks with Done, keeping order and appending new apps", () => {
    render(<ManagerRoot />);
    const dialog = openPicker();
    fireEvent.click(box(dialog, "Visual Studio Code"));
    fireEvent.click(box(dialog, "GitHub Desktop"));
    fireEvent.click(box(dialog, "Terminal"));
    expect(within(dialog).getByText("3 selected")).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Done" }));
    expect(calls("set_group_apps")).toEqual([{ id: "g1", appIds: ["hid", "np", "vsc", "git"] }]);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(tileNames()).toEqual(["Notepad", "Visual Studio Code", "GitHub Desktop"]);
  });

  it("changes nothing on Cancel, Escape, or Done without changes", () => {
    render(<ManagerRoot />);
    fireEvent.click(box(openPicker(), "Visual Studio Code"));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.keyDown(openPicker(), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    fireEvent.click(within(openPicker()).getByRole("button", { name: "Done" }));
    expect(calls("set_group_apps")).toEqual([]);
  });

  it("ticks programs added with Add other program", async () => {
    invokeMock.mockImplementation((command) =>
      Promise.resolve(command === "pick_and_add_apps" ? [{ id: "vsc" }] : null),
    );
    render(<ManagerRoot />);
    const dialog = openPicker();
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Add other program…" }));
    });
    expect(calls("pick_and_add_apps")).toHaveLength(1);
    expect(box(dialog, "Visual Studio Code").checked).toBe(true);
  });

  it("shows progress while the Start Menu is rescanned", async () => {
    let finish: (found: number) => void = () => undefined;
    invokeMock.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
    render(<ManagerRoot />);
    const dialog = openPicker();
    fireEvent.click(within(dialog).getByRole("button", { name: "Rescan Start Menu" }));
    const busy = within(dialog).getByRole("button", { name: "Scanning…" }) as HTMLButtonElement;
    expect(busy.disabled).toBe(true);
    expect(within(dialog).getByText("Scanning the Start Menu…")).toBeTruthy();
    await act(async () => finish(42));
    expect(within(dialog).getByRole("button", { name: "Rescan Start Menu" })).toBeTruthy();
    expect(useToastStore.getState().toasts[0]?.title).toBe("Start Menu scanned. Found 42 apps.");
  });

  it("renders only part of a list longer than 300 apps", () => {
    const apps = Array.from({ length: 400 }, (_, i) => ({
      id: `a${i}`,
      name: `App ${String(i).padStart(3, "0")}`,
    }));
    useAppStore.getState().applyState({ ...STATE, apps });
    render(<ManagerRoot />);
    const rows = within(openPicker()).getAllByRole("checkbox");
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(60);
    expect(rows[0]?.closest("li")?.getAttribute("aria-setsize")).toBe("400");
  });
});

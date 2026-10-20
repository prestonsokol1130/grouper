import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { AppState } from "../../bindings";
import { useAppStore } from "../../store/appStore";
import { useToastStore } from "../../store/toastStore";
import { invokeMock } from "../../test/tauriMock";
import { checkGroupName } from "./groupName";
import { ManagerRoot } from "./ManagerRoot";

const STATE: AppState = {
  // Past the first-run welcome, which otherwise replaces the Manager.
  settings: { firstRunDone: true },
  apps: [
    { id: "a", name: "Alpha" },
    { id: "b", name: "Beta" },
  ],
  groups: [
    { id: "g1", name: "Work", appIds: ["a", "b"] },
    { id: "g2", name: "Games", appIds: [] },
    { id: "g3", name: "Media", appIds: ["b"] },
  ],
};

const calls = (command: string) =>
  invokeMock.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

const rows = () =>
  within(screen.getByRole("navigation", { name: "Groups" })).getAllByRole("button");

beforeEach(() => {
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
});

describe("checkGroupName", () => {
  it("trims a name and rejects an empty one", () => {
    expect(checkGroupName("  Work  ")).toEqual({ ok: true, name: "Work" });
    expect(checkGroupName("")).toMatchObject({ ok: false });
    expect(checkGroupName("   \t")).toMatchObject({ ok: false });
  });
});

describe("Manager layout", () => {
  it("lists groups in order with previews and selects the first", () => {
    render(<ManagerRoot />);
    expect(rows().map((row) => row.textContent)).toEqual(["Work", "Games", "Media"]);
    expect(rows()[0]?.getAttribute("aria-current")).toBe("page");
    const nav = screen.getByRole("navigation", { name: "Groups" });
    expect(within(nav).getAllByTestId("folder-art")).toHaveLength(3);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Work");
    expect(screen.getByRole("list", { name: "Apps in Work" }).children).toHaveLength(2);
  });

  it("shows the empty state when there are no groups", () => {
    useAppStore.getState().applyState({ ...STATE, groups: [] });
    render(<ManagerRoot />);
    expect(screen.getByText("No groups yet.")).toBeTruthy();
    expect(screen.getByText("Click New group to make your first one.")).toBeTruthy();
  });

  it("opens Settings from the bottom of the sidebar", () => {
    render(<ManagerRoot />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("region", { name: "Settings" })).toBeTruthy();
    expect(screen.getByText("Launchpad shortcut")).toBeTruthy();
  });
});

describe("rename", () => {
  const startRename = () => {
    render(<ManagerRoot />);
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    return screen.getByRole("textbox", { name: "Group name" }) as HTMLInputElement;
  };

  it("saves the trimmed name on Enter", () => {
    const input = startRename();
    expect(document.activeElement).toBe(input);
    fireEvent.change(input, { target: { value: "  Office  " } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(calls("rename_group")).toEqual([{ id: "g1", name: "Office" }]);
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Office");
  });

  it("refuses an empty name and keeps the box open", () => {
    const input = startRename();
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByRole("alert").textContent).toBe("Give the group a name.");
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(calls("rename_group")).toEqual([]);
    fireEvent.blur(input);
    expect(calls("rename_group")).toEqual([]);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Work");
  });

  it("cancels on Escape", () => {
    const input = startRename();
    fireEvent.change(input, { target: { value: "Something else" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(calls("rename_group")).toEqual([]);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Work");
  });

  it("creates New Group and opens its rename box", async () => {
    invokeMock.mockImplementation((command) =>
      Promise.resolve(command === "create_group" ? { id: "g4", name: "New Group" } : null),
    );
    render(<ManagerRoot />);
    await act(async () => {
      fireEvent.click(screen.getAllByRole("button", { name: "New group" })[0] as HTMLElement);
    });
    expect(calls("create_group")).toEqual([{ name: "New Group" }]);
    act(() => {
      useAppStore.getState().applyState({
        ...STATE,
        groups: [...(STATE.groups ?? []), { id: "g4", name: "New Group", appIds: [] }],
      });
    });
    const input = screen.getByRole("textbox", { name: "Group name" }) as HTMLInputElement;
    expect(input.value).toBe("New Group");
    expect(document.activeElement).toBe(input);
  });
});

describe("delete", () => {
  it("asks first, warns about the taskbar shortcut, and deletes on confirm", async () => {
    render(<ManagerRoot />);
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    const dialog = screen.getByRole("alertdialog", { name: "Delete Work?" });
    expect(dialog.textContent).toContain("taskbar shortcut will be removed too");
    expect(document.activeElement?.textContent).toBe("Cancel");

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
    expect(calls("delete_group")).toEqual([]);

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    fireEvent.keyDown(screen.getByRole("alertdialog"), { key: "Escape" });
    expect(screen.queryByRole("alertdialog")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await act(async () => {
      fireEvent.click(
        within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }),
      );
    });
    expect(calls("delete_group")).toEqual([{ id: "g1" }]);
  });

  it("shows a toast when the delete fails", async () => {
    invokeMock.mockRejectedValue("Group not found: g1");
    render(<ManagerRoot />);
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await act(async () => {
      fireEvent.click(
        within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }),
      );
    });
    expect(useToastStore.getState().toasts[0]?.title).toBe("Couldn't delete the group.");
  });
});

describe("reorder", () => {
  it("moves a group with Alt+Down and Alt+Up", () => {
    render(<ManagerRoot />);
    fireEvent.keyDown(rows()[0] as HTMLElement, { key: "ArrowDown", altKey: true });
    expect(calls("reorder_groups")).toEqual([{ ids: ["g2", "g1", "g3"] }]);
    expect(rows().map((row) => row.textContent)).toEqual(["Games", "Work", "Media"]);
    fireEvent.keyDown(rows()[0] as HTMLElement, { key: "ArrowUp", altKey: true });
    expect(calls("reorder_groups")).toHaveLength(1);
  });

  it("moves a group by dragging it", () => {
    render(<ManagerRoot />);
    const items = screen.getByRole("navigation", { name: "Groups" }).querySelectorAll("li");
    items.forEach((item, index) => {
      item.getBoundingClientRect = () => ({ top: index * 48, height: 44 }) as DOMRect;
    });
    const first = rows()[0] as HTMLElement;
    fireEvent.pointerDown(first, { button: 0, clientY: 10, pointerId: 1 });
    fireEvent.pointerMove(first, { clientY: 110, pointerId: 1 });
    expect(first.getAttribute("data-dragging")).toBe("true");
    // Rows only shift while dragging; the DOM order (and capture) stays.
    expect(rows()[0]).toBe(first);
    expect(first.style.translate).toBe("0 100px");
    expect((rows()[2] as HTMLElement).style.translate).toBe("0 -48px");
    fireEvent.pointerUp(first, { clientY: 110, pointerId: 1 });
    fireEvent.click(first);
    expect(calls("reorder_groups")).toEqual([{ ids: ["g2", "g3", "g1"] }]);
    // The click that ends a drag does not select the row.
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Work");
  });

  it("does not reorder on a plain click", () => {
    render(<ManagerRoot />);
    const second = rows()[1] as HTMLElement;
    fireEvent.pointerDown(second, { button: 0, clientY: 60, pointerId: 1 });
    fireEvent.pointerUp(second, { clientY: 61, pointerId: 1 });
    fireEvent.click(second);
    expect(calls("reorder_groups")).toEqual([]);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Games");
  });
});

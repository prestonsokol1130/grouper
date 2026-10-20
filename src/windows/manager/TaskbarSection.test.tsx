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
    { id: "a", name: "Alpha" },
    { id: "b", name: "Beta" },
    { id: "c", name: "Gamma", missing: true },
  ],
  groups: [{ id: "g1", name: "Coding", appIds: ["a", "b", "c"] }],
};

const calls = (command: string) =>
  invokeMock.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

const card = () => screen.getByRole("region", { name: "Taskbar" });

beforeEach(() => {
  useAppStore.getState().applyState(STATE);
  useToastStore.setState({ toasts: [] });
  invokeMock.mockResolvedValue(null);
});

describe("taskbar section", () => {
  it("offers to create the shortcut when there is none", async () => {
    let finish: (path: string) => void = () => undefined;
    invokeMock.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
    render(<ManagerRoot />);
    expect(within(card()).queryByRole("button", { name: /Show in Explorer/ })).toBeNull();
    fireEvent.click(within(card()).getByRole("button", { name: "Create taskbar shortcut" }));
    expect(calls("create_group_shortcut")).toEqual([{ groupId: "g1" }]);
    const busy = within(card()).getByRole("button", { name: "Creating shortcut…" });
    expect((busy as HTMLButtonElement).disabled).toBe(true);
    await act(async () => finish("C:\\Grouper\\Coding.lnk"));
    expect(within(card()).getByRole("button", { name: "Create taskbar shortcut" })).toBeTruthy();
  });

  it("shows Show in Explorer and the manual pin steps once it exists", () => {
    useAppStore.getState().applyState({
      ...STATE,
      groups: [{ id: "g1", name: "Coding", appIds: ["a"], hasShortcut: true }],
    });
    render(<ManagerRoot />);
    expect(within(card()).queryByRole("button", { name: "Create taskbar shortcut" })).toBeNull();
    fireEvent.click(within(card()).getByRole("button", { name: "Show in Explorer" }));
    expect(calls("reveal_group_shortcut")).toEqual([{ groupId: "g1" }]);
    const text = card().textContent ?? "";
    expect(text).toContain("Windows doesn't let apps pin themselves");
    expect(text).toContain("Right-click Coding.");
    expect(text).toContain("choose Show more options");
    expect(text).toContain("Choose Pin to taskbar.");
  });

  it("says so when the shortcut can't be made", async () => {
    invokeMock.mockRejectedValue("icon build failed");
    render(<ManagerRoot />);
    await act(async () => {
      fireEvent.click(within(card()).getByRole("button", { name: "Create taskbar shortcut" }));
    });
    expect(useToastStore.getState().toasts[0]?.title).toBe("Couldn't create the shortcut.");
  });

  it("previews the folder icon live as apps change", () => {
    render(<ManagerRoot />);
    const art = () => within(card()).getByTestId("folder-art");
    // Missing apps are left out, like the taskbar .ico.
    expect(art().children).toHaveLength(2);
    act(() => {
      useAppStore.getState().applyState({
        ...STATE,
        groups: [{ id: "g1", name: "Coding", appIds: ["b"] }],
      });
    });
    expect(art().children).toHaveLength(1);
  });
});

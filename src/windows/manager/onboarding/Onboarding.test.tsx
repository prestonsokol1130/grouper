import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { AppState, Group } from "../../../bindings";
import { useAppStore } from "../../../store/appStore";
import { invokeMock } from "../../../test/tauriMock";
import { ManagerRoot } from "../ManagerRoot";
import { nextStep, previousStep, STEPS, stepLabel } from "./onboardingSteps";

const STATE: AppState = {
  apps: [
    { id: "n", name: "Notepad" },
    { id: "c", name: "Calculator" },
  ],
  groups: [],
  settings: { firstRunDone: false },
};

const calls = (command: string) =>
  invokeMock.mock.calls.filter(([name]) => name === command).map(([, args]) => args);

const next = () => fireEvent.click(screen.getByRole("button", { name: /^(Next|Skip|Done)$/ }));

beforeEach(() => {
  useAppStore.getState().applyState(STATE);
  invokeMock.mockImplementation((command, args) => {
    if (command === "rescan_apps") return Promise.resolve(2);
    if (command === "create_group") {
      const group: Group = { id: "g1", name: (args as { name: string }).name, appIds: [] };
      return Promise.resolve(group);
    }
    return Promise.resolve(null);
  });
});

describe("step order", () => {
  it("goes welcome, scan, group, pin and then finishes", () => {
    expect(STEPS).toEqual(["welcome", "scan", "group", "pin"]);
    expect(nextStep("welcome")).toBe("scan");
    expect(nextStep("pin")).toBeNull();
    expect(previousStep("scan")).toBe("welcome");
    expect(previousStep("welcome")).toBeNull();
    expect(stepLabel("group")).toBe("Step 3 of 4");
  });
});

describe("first run", () => {
  it("replaces the Manager until first_run_done is set", () => {
    render(<ManagerRoot />);
    expect(screen.getByRole("heading", { name: "Welcome to Grouper" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "Alt + `" })).toBeTruthy();
    expect(screen.queryByRole("navigation", { name: "Groups" })).toBeNull();
    act(() => useAppStore.getState().applyState({ ...STATE, settings: { firstRunDone: true } }));
    expect(screen.getByRole("navigation", { name: "Groups" })).toBeTruthy();
  });

  it("moves forward and back through the steps", async () => {
    render(<ManagerRoot />);
    expect(screen.getByText("Step 1 of 4")).toBeTruthy();
    await act(async () => next());
    expect(screen.getByRole("heading", { name: "Scanning your apps" })).toBeTruthy();
    expect(calls("rescan_apps")).toHaveLength(1);
    expect(screen.getByText(/Found 2 apps/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByText("Step 1 of 4")).toBeTruthy();
    await act(async () => next());
    // Coming back does not scan again.
    expect(calls("rescan_apps")).toHaveLength(1);
    next();
    expect(screen.getByRole("heading", { name: "Make your first group" })).toBeTruthy();
    next();
    expect(screen.getByRole("heading", { name: "One last step" })).toBeTruthy();
    expect(calls("update_settings")).toEqual([]);
    await act(async () => next());
    expect(calls("update_settings")).toEqual([{ patch: { firstRunDone: true } }]);
  });

  it("skips the whole setup and saves the flag", async () => {
    render(<ManagerRoot />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Skip setup" }));
    });
    expect(calls("update_settings")).toEqual([{ patch: { firstRunDone: true } }]);
  });

  it("makes the group, opens the app picker and needs apps for the shortcut", async () => {
    render(<ManagerRoot />);
    await act(async () => next());
    next();
    const create = screen.getByRole("button", { name: "Create taskbar shortcut" });
    expect(create).toHaveProperty("disabled", true);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Choose apps/ }));
    });
    expect(screen.getByText("Give the group a name.")).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Group name" }), {
      target: { value: "  Work " },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Choose apps/ }));
    });
    expect(calls("create_group")).toEqual([{ name: "Work" }]);
    expect(screen.getByRole("dialog", { name: "Add apps to Work" })).toBeTruthy();
  });
});

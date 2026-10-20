import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { showToast, useToastStore } from "../store/toastStore";
import { TOAST_TIMEOUT_MS, Toaster } from "./Toaster";
import { miniLayout } from "./folderLayout";

beforeEach(() => {
  vi.useFakeTimers();
  useToastStore.setState({ toasts: [] });
});

afterEach(() => vi.useRealTimers());

describe("Toaster", () => {
  it("shows a toast and closes it after five seconds", () => {
    render(<Toaster />);
    act(() => {
      showToast({ kind: "error", title: "Couldn't rename the group." });
    });
    expect(screen.getByRole("alert").textContent).toContain("Couldn't rename the group.");
    act(() => {
      vi.advanceTimersByTime(TOAST_TIMEOUT_MS);
    });
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("keeps a toast with an action until it is used", () => {
    const run = vi.fn();
    render(<Toaster />);
    act(() => {
      showToast({ kind: "info", title: "Shortcut taken", action: { label: "Fix", run } });
    });
    act(() => {
      vi.advanceTimersByTime(TOAST_TIMEOUT_MS * 2);
    });
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    expect(run).toHaveBeenCalledOnce();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("closes from its close button", () => {
    render(<Toaster />);
    act(() => {
      showToast({ kind: "success", title: "Done" });
    });
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("status")).toBeNull();
  });
});

describe("miniLayout", () => {
  it("matches the taskbar icon layout for 0 to 5 apps", () => {
    expect(miniLayout(0)).toEqual([]);
    expect(miniLayout(1)).toHaveLength(1);
    expect(miniLayout(1)[0]?.size).toBeCloseTo(0.33 * 1.6);
    const [left, right] = miniLayout(2);
    expect(left?.y).toBeCloseTo((1 - 0.33) / 2);
    expect(right?.x).toBeCloseTo(0.14 + 0.33 + 0.06);
    expect(miniLayout(3)[2]?.x).toBeCloseTo((1 - 0.33) / 2);
    expect(miniLayout(5)).toHaveLength(4);
  });
});

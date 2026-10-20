import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useToastStore } from "../store/toastStore";
import { emitMock } from "../test/tauriMock";
import { displayHotkey, followHotkeyFailures } from "./hotkeyToast";

const flush = () => Promise.resolve();

beforeEach(() => {
  useToastStore.setState({ toasts: [] });
  vi.spyOn(console, "warn").mockImplementation(() => undefined);
});

afterEach(() => vi.restoreAllMocks());

describe("displayHotkey", () => {
  it("spaces out the parts like DESIGN.md", () => {
    expect(displayHotkey("Alt+`")).toBe("Alt + `");
    expect(displayHotkey("Ctrl+Alt+Space")).toBe("Ctrl + Alt + Space");
  });
});

describe("followHotkeyFailures", () => {
  it("shows the DESIGN.md toast once, keeping the raw error for the log", async () => {
    const stop = followHotkeyFailures();
    await flush();
    const failed = { hotkey: "Alt+`", error: "HotKey already registered" };
    emitMock("hotkey:failed", failed);
    emitMock("hotkey:failed", failed);
    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0]?.title).toBe("Alt + ` is already used by another app.");
    expect(toasts[0]?.message).toBe("Pick a different shortcut in Settings.");
    expect(console.warn).toHaveBeenCalledWith("hotkey: HotKey already registered");
    stop();
  });
});

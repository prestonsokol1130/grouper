import { beforeEach, describe, expect, it } from "vitest";
import { useToastStore } from "../../store/toastStore";
import { CARRY_MS, setLaunchpadVisible, showOrKeep, takePending } from "./pendingToast";

const TOAST = { kind: "error" as const, title: "Couldn't open Spotify." };

beforeEach(() => {
  useToastStore.setState({ toasts: [] });
  takePending();
});

describe("toasts kept for the next Launchpad open", () => {
  it("shows at once while the Launchpad is showing", () => {
    setLaunchpadVisible(true);
    showOrKeep(TOAST, 0);
    expect(useToastStore.getState().toasts.map((toast) => toast.title)).toEqual([TOAST.title]);
    expect(takePending(0)).toBeNull();
  });

  it("keeps one that comes while hidden, once, for up to CARRY_MS", () => {
    setLaunchpadVisible(false);
    showOrKeep(TOAST, 1000);
    expect(useToastStore.getState().toasts).toEqual([]);
    expect(takePending(1000 + CARRY_MS)).toEqual(TOAST);
    expect(takePending(1000 + CARRY_MS)).toBeNull();
  });

  it("drops one that is older than CARRY_MS", () => {
    setLaunchpadVisible(false);
    showOrKeep(TOAST, 1000);
    expect(takePending(1001 + CARRY_MS)).toBeNull();
  });
});

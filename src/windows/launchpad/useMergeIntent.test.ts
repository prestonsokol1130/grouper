import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MERGE_HOLD_MS } from "./mergeIntent";
import { useMergeIntent } from "./useMergeIntent";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

const advance = (ms: number) => act(() => vi.advanceTimersByTime(ms));

describe("useMergeIntent", () => {
  it("switches to merge after resting MERGE_HOLD_MS over one centre", () => {
    const { result } = renderHook(() => useMergeIntent());
    act(() => result.current.hover("app:b"));
    advance(MERGE_HOLD_MS - 1);
    expect(result.current.target).toBeNull();
    // Moving within the same centre does not restart the wait.
    act(() => result.current.hover("app:b"));
    advance(1);
    expect(result.current.target).toBe("app:b");
  });

  it("starts the wait again on another tile", () => {
    const { result } = renderHook(() => useMergeIntent());
    act(() => result.current.hover("app:b"));
    advance(300);
    act(() => result.current.hover("folder:g"));
    advance(300);
    expect(result.current.target).toBeNull();
    advance(100);
    expect(result.current.target).toBe("folder:g");
  });

  it("goes straight back to reorder when the pointer leaves the centre", () => {
    const { result } = renderHook(() => useMergeIntent());
    act(() => result.current.hover("app:b"));
    advance(MERGE_HOLD_MS);
    act(() => result.current.hover(null));
    expect(result.current.target).toBeNull();
    // Leaving before the wait is over cancels it.
    act(() => result.current.hover("app:c"));
    advance(200);
    act(() => result.current.hover(null));
    advance(MERGE_HOLD_MS);
    expect(result.current.target).toBeNull();
  });

  it("keeps the target for the drop on stop, clears it on reset", () => {
    const { result } = renderHook(() => useMergeIntent());
    act(() => result.current.hover("app:b"));
    advance(MERGE_HOLD_MS);
    act(() => result.current.stop());
    expect(result.current.target).toBe("app:b");
    act(() => result.current.reset());
    expect(result.current.target).toBeNull();
  });

  it("never fires after unmount", () => {
    const { result, unmount } = renderHook(() => useMergeIntent());
    act(() => result.current.hover("app:b"));
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});

import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EDGE_FLIP_MS, edgeSide, useEdgeFlip } from "./useEdgeFlip";

afterEach(() => vi.useRealTimers());

describe("edgeSide", () => {
  it("is -1 or 1 within the zone of the left or right edge", () => {
    expect(edgeSide(10, 1000)).toBe(-1);
    expect(edgeSide(48, 1000)).toBe(-1);
    expect(edgeSide(500, 1000)).toBe(0);
    expect(edgeSide(960, 1000)).toBe(1);
    expect(edgeSide(10, 0)).toBe(0);
  });
});

describe("useEdgeFlip", () => {
  it("turns after a rest at an edge, again while it stays, and stops on leaving", () => {
    vi.useFakeTimers();
    const turn = vi.fn();
    const { result } = renderHook(() => useEdgeFlip(turn));
    const right = window.innerWidth - 10;
    act(() => result.current(right));
    act(() => vi.advanceTimersByTime(EDGE_FLIP_MS - 1));
    expect(turn).not.toHaveBeenCalled();
    // Moving within the zone keeps the timer going.
    act(() => result.current(right + 5));
    act(() => vi.advanceTimersByTime(1));
    expect(turn).toHaveBeenLastCalledWith(1);
    act(() => vi.advanceTimersByTime(EDGE_FLIP_MS));
    expect(turn).toHaveBeenCalledTimes(2);
    act(() => result.current(window.innerWidth / 2));
    act(() => vi.advanceTimersByTime(EDGE_FLIP_MS * 3));
    expect(turn).toHaveBeenCalledTimes(2);
    act(() => result.current(5));
    act(() => vi.advanceTimersByTime(EDGE_FLIP_MS));
    expect(turn).toHaveBeenLastCalledWith(-1);
    act(() => result.current(null));
    act(() => vi.advanceTimersByTime(EDGE_FLIP_MS * 3));
    expect(turn).toHaveBeenCalledTimes(3);
  });
});

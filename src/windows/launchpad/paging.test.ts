import { describe, expect, it } from "vitest";
import { clampPage, dragStep, WheelPager } from "./paging";

describe("WheelPager", () => {
  it("turns one page per mouse notch", () => {
    const wheel = new WheelPager();
    expect(wheel.step(0, 100, 0)).toBe(1);
    expect(wheel.step(0, 100, 400)).toBe(1);
    expect(wheel.step(0, -100, 800)).toBe(-1);
  });

  it("turns once for a whole trackpad swipe and its momentum", () => {
    const wheel = new WheelPager();
    const steps = Array.from({ length: 40 }, (_, i) => wheel.step(0, 30 - i * 0.5, i * 16));
    expect(steps.filter((s) => s !== 0)).toEqual([1]);
    // A pause ends the gesture; the next swipe turns again.
    expect(wheel.step(0, 30, 40 * 16 + 400)).toBe(1);
  });

  it("pages on a horizontal wheel too, right going forward", () => {
    const wheel = new WheelPager();
    expect(wheel.step(-60, 5, 0)).toBe(-1);
    expect(wheel.step(60, 0, 1000)).toBe(1);
  });

  it("ignores tiny deltas", () => {
    const wheel = new WheelPager();
    expect(wheel.step(1, 2, 0)).toBe(0);
    expect(wheel.step(0, 50, 50)).toBe(1);
  });
});

describe("dragStep", () => {
  it("turns when dragged a quarter of the page, left going forward", () => {
    expect(dragStep(-300, 0, 1200)).toBe(1);
    expect(dragStep(300, 0, 1200)).toBe(-1);
    expect(dragStep(-200, 0, 1200)).toBe(0);
  });

  it("turns on a quick flick, but not a twitch or a flick back", () => {
    expect(dragStep(-60, -0.8, 1200)).toBe(1);
    expect(dragStep(10, 2, 1200)).toBe(0);
    expect(dragStep(-60, 0.8, 1200)).toBe(0);
  });
});

describe("clampPage", () => {
  it("keeps a page in range", () => {
    expect(clampPage(-1, 3)).toBe(0);
    expect(clampPage(5, 3)).toBe(2);
    expect(clampPage(1, 0)).toBe(0);
  });
});

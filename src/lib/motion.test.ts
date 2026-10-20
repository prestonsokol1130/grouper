import { describe, expect, it } from "vitest";
import {
  closeTiming,
  FLUENT_ACCELERATE,
  FLUENT_DECELERATE,
  OPEN_CLOSE,
  openCloseTiming,
  REDUCED_FADE,
  reversedEasing,
} from "./motion";

describe("motion presets", () => {
  it("open on the Fluent decelerate curve in 200 to 300ms", () => {
    for (const preset of Object.values(OPEN_CLOSE)) {
      expect(preset.easing).toBe(FLUENT_DECELERATE);
      expect(preset.durationMs).toBeGreaterThanOrEqual(200);
      expect(preset.durationMs).toBeLessThanOrEqual(300);
    }
    expect(OPEN_CLOSE.genie.durationMs).toBe(300);
    expect(OPEN_CLOSE.smooth).toMatchObject({ durationMs: 200, slidePx: 12 });
  });

  it("close quicker, accelerating away", () => {
    for (const style of ["genie", "scale", "smooth"] as const) {
      const closing = closeTiming(style, false);
      expect(closing.easing).toBe(FLUENT_ACCELERATE);
      expect(closing.durationMs).toBeGreaterThanOrEqual(150);
      expect(closing.durationMs).toBeLessThanOrEqual(200);
      expect(closing.durationMs).toBeLessThan(openCloseTiming(style, false).durationMs);
    }
  });

  it("become the 120ms fade with reduced motion", () => {
    expect(openCloseTiming("genie", true)).toBe(REDUCED_FADE);
    expect(closeTiming("genie", true)).toBe(REDUCED_FADE);
  });
});

describe("reversedEasing", () => {
  it("mirrors a cubic-bezier so a reversed animation moves like it", () => {
    expect(reversedEasing(FLUENT_ACCELERATE)).toBe("cubic-bezier(0, 0.5, 0.3, 1)");
    expect(reversedEasing("cubic-bezier(0.1,0.9,0.2,1)")).toBe("cubic-bezier(0.8, 0, 0.9, 0.1)");
    // Mirroring twice gives the curve back.
    expect(reversedEasing(reversedEasing(FLUENT_DECELERATE))).toBe(FLUENT_DECELERATE);
  });

  it("leaves keywords and broken values alone", () => {
    expect(reversedEasing("linear")).toBe("linear");
    expect(reversedEasing("cubic-bezier(1, 2)")).toBe("cubic-bezier(1, 2)");
    expect(reversedEasing("cubic-bezier(a, b, c, d)")).toBe("cubic-bezier(a, b, c, d)");
  });
});

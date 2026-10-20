import { describe, expect, it } from "vitest";
import { invokeMock } from "../test/tauriMock";
import {
  accentShades,
  applyMaterial,
  applyTheme,
  FALLBACK_ACCENT,
  parseHex,
  setAccent,
} from "./theme";

describe("accent shades", () => {
  it("parses #RRGGBB only", () => {
    expect(parseHex("#0067C0")).toEqual([0, 103, 192]);
    expect(parseHex("0067C0")).toBeNull();
    expect(parseHex("#fff")).toBeNull();
  });

  it("derives hover 90% and pressed 80% opacity", () => {
    expect(accentShades("#0067C0")).toEqual({
      accent: "rgb(0, 103, 192)",
      hover: "rgba(0, 103, 192, 0.9)",
      pressed: "rgba(0, 103, 192, 0.8)",
      onAccent: "#FFFFFF",
    });
  });

  it("puts black text on light accents", () => {
    expect(accentShades("#4CC2FF")?.onAccent).toBe("#000000");
    expect(accentShades("#FFFF00")?.onAccent).toBe("#000000");
    expect(accentShades("#000080")?.onAccent).toBe("#FFFFFF");
  });

  it("writes the accent for the current theme onto <html>", () => {
    applyTheme();
    setAccent({ light: "#112233", dark: "#DDEEFF" });
    const style = document.documentElement.style;
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(style.getPropertyValue("--accent")).toBe("rgb(17, 34, 51)");
    setAccent(null);
    expect(FALLBACK_ACCENT.light).toBe("#0067C0");
    expect(style.getPropertyValue("--accent")).toBe("rgb(0, 103, 192)");
  });
});

describe("applyMaterial", () => {
  it("uses the window's material, solid when unknown", async () => {
    invokeMock.mockResolvedValueOnce("mica");
    await expect(applyMaterial()).resolves.toBe("mica");
    expect(document.documentElement.dataset.material).toBe("mica");
    invokeMock.mockRejectedValueOnce(new Error("no tauri"));
    await expect(applyMaterial()).resolves.toBe("solid");
  });
});

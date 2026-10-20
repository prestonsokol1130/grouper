import { describe, expect, it } from "vitest";
import { followSettings } from "../lib/followSettings";
import { setAccent } from "../lib/theme";
import { launchpadDensity, storedPerPage } from "../windows/launchpad/launchpadGrid";
import { useAppStore } from "./appStore";
import { DEFAULT_SETTINGS, settingsOf } from "./settings";

describe("settings", () => {
  it("fills every missing field with the DESIGN.md default", () => {
    expect(settingsOf(null)).toEqual(DEFAULT_SETTINGS);
    const settings = settingsOf({
      settings: { iconSize: "large", hotCorners: { topLeft: false }, accent: null as never },
    });
    expect(settings.iconSize).toBe("large");
    expect(settings.hotCorners).toEqual({ topLeft: false, topRight: true });
    expect(settings.accent).toBe("windows");
    expect(settings.gridColumns).toBe(7);
  });

  it("drives the Launchpad grid and page size", () => {
    useAppStore.getState().applyState({ settings: {} });
    expect(launchpadDensity()).toEqual({ columns: 7, rows: 5, iconPx: 72 });
    useAppStore.getState().applyState({
      settings: { gridColumns: 8, gridRows: 4, iconSize: "small" },
    });
    expect(launchpadDensity()).toEqual({ columns: 8, rows: 4, iconPx: 56 });
    expect(storedPerPage()).toBe(32);
  });

  it("paints a custom accent and goes back to the Windows one", () => {
    setAccent({ light: "#112233", dark: "#112233" });
    useAppStore.getState().applyState({ settings: { accent: "#FF8800" } });
    const stop = followSettings();
    const accent = () => document.documentElement.style.getPropertyValue("--accent");
    expect(accent()).toBe("rgb(255, 136, 0)");
    useAppStore.getState().applyState({ settings: { accent: "windows" } });
    expect(accent()).toBe("rgb(17, 34, 51)");
    stop();
  });
});

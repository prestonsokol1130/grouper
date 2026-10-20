import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppState } from "../bindings";
import { setPreviewIconUrls } from "../lib/iconUrl";
import { useAppStore } from "../store/appStore";
import { AppIcon } from "./AppIcon";
import { AppTile } from "./AppTile";
import { FolderTile } from "./FolderTile";

const LONG = "A very long application name that cannot fit";

const STATE: AppState = {
  apps: [
    { id: "a", name: "Alpha" },
    { id: "b", name: "Beta" },
    { id: "c", name: "Gamma" },
    { id: "d", name: "Delta" },
    { id: "e", name: "Epsilon" },
    { id: "long", name: LONG },
    { id: "gone", name: "Spotify", missing: true },
  ],
  groups: [
    { id: "five", name: "Five", appIds: ["a", "b", "c", "d", "e"] },
    { id: "skip", name: "Skip", appIds: ["gone", "a"] },
  ],
};

beforeEach(() => {
  setPreviewIconUrls(
    new Map([
      ["a", "a.png"],
      ["b", "b.png"],
      ["c", "c.png"],
      ["d", "d.png"],
      ["e", "e.png"],
      ["long", "long.png"],
      ["_fallback", "fallback.png"],
    ]),
  );
  useAppStore.getState().applyState(STATE);
});

describe("AppTile", () => {
  it("shows the name on one truncated line with the full name as tooltip", () => {
    render(<AppTile appId="long" />);
    const label = screen.getByTestId("tile-label");
    expect(label.textContent).toBe(LONG);
    expect(label.className).toContain("truncate");
    expect(screen.getByRole("button").getAttribute("title")).toBe(LONG);
  });

  it("is a focusable button activated once by Enter, Space and click", () => {
    const onActivate = vi.fn();
    render(<AppTile appId="a" onActivate={onActivate} />);
    const tile = screen.getByRole("button", { name: "Alpha" });
    tile.focus();
    expect(document.activeElement).toBe(tile);
    fireEvent.keyDown(tile, { key: "Enter" });
    fireEvent.keyDown(tile, { key: " " });
    fireEvent.keyUp(tile, { key: " " });
    fireEvent.click(tile);
    expect(onActivate).toHaveBeenCalledTimes(3);
    expect(onActivate).toHaveBeenCalledWith("a");
  });

  it("ignores held-down key repeats and other keys", () => {
    const onActivate = vi.fn();
    render(<AppTile appId="a" onActivate={onActivate} />);
    const tile = screen.getByRole("button");
    fireEvent.keyDown(tile, { key: "Enter", repeat: true });
    fireEvent.keyDown(tile, { key: "a" });
    expect(onActivate).not.toHaveBeenCalled();
  });

  it("does not activate when disabled", () => {
    const onActivate = vi.fn();
    render(<AppTile appId="a" onActivate={onActivate} disabled />);
    const tile = screen.getByRole("button");
    fireEvent.keyDown(tile, { key: "Enter" });
    fireEvent.click(tile);
    expect(onActivate).not.toHaveBeenCalled();
  });

  it("dims and explains a missing app", () => {
    render(<AppTile appId="gone" />);
    const tile = screen.getByRole("button");
    expect(tile.getAttribute("title")).toContain("can't be found");
    expect(tile.querySelector('[data-missing="true"]')).not.toBeNull();
  });
});

describe("AppIcon", () => {
  it("has a fixed size, async decoding and no dragging", () => {
    const { container } = render(<AppIcon appId="a" size={48} />);
    const img = container.querySelector("img");
    expect(img?.getAttribute("width")).toBe("48");
    expect(img?.getAttribute("height")).toBe("48");
    expect(img?.getAttribute("decoding")).toBe("async");
    expect(img?.getAttribute("draggable")).toBe("false");
  });

  it("switches to the fallback icon when the image fails", () => {
    const { container } = render(<AppIcon appId="a" size={48} />);
    const img = () => container.querySelector("img");
    expect(img()?.getAttribute("src")).toBe("a.png");
    act(() => {
      img()?.dispatchEvent(new Event("error"));
    });
    expect(img()?.getAttribute("src")).toBe("fallback.png");
  });

  it("keeps an empty box of the same size when even the fallback fails", () => {
    const { container } = render(<AppIcon appId="a" size={40} />);
    for (let i = 0; i < 2; i += 1) {
      act(() => {
        container.querySelector("img")?.dispatchEvent(new Event("error"));
      });
    }
    expect(container.querySelector("img")).toBeNull();
    const box = container.firstElementChild as HTMLElement;
    expect([box.style.width, box.style.height]).toEqual(["40px", "40px"]);
  });
});

describe("FolderTile", () => {
  it("shows at most four mini icons and the group name", () => {
    const onOpen = vi.fn();
    render(<FolderTile groupId="five" onOpen={onOpen} />);
    const art = screen.getByTestId("folder-art");
    expect(art.querySelectorAll("img")).toHaveLength(4);
    expect(screen.getByTestId("tile-label").textContent).toBe("Five");
    fireEvent.keyDown(screen.getByRole("button", { name: "Five" }), { key: "Enter" });
    expect(onOpen).toHaveBeenCalledWith("five");
  });

  it("skips missing apps like the taskbar icon", () => {
    render(<FolderTile groupId="skip" />);
    const imgs = screen.getByTestId("folder-art").querySelectorAll("img");
    expect([...imgs].map((img) => img.getAttribute("src"))).toEqual(["a.png"]);
  });

  it("uses the squircle clip path when asked", () => {
    render(<FolderTile groupId="five" shape="squircle" />);
    expect(screen.getByTestId("folder-art").style.clipPath).toContain("polygon(");
  });
});

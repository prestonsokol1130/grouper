import { act, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AppIcon } from "../components/AppIcon";
import { useAppStore } from "../store/appStore";
import { emitMock, invokeMock } from "../test/tauriMock";
import { iconUrl, initIconUrls } from "./iconUrl";

/** Lets pending promise callbacks run. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
const version = () => useAppStore.getState().iconVersion;

describe("icon cache-buster", () => {
  it("bumps once per icons:updated, even while get_cache_dir is pending", async () => {
    let resolveDir: (dir: string) => void = () => undefined;
    invokeMock.mockReturnValueOnce(new Promise((r) => (resolveDir = r)));
    const ready = initIconUrls();
    await flush();
    const before = version();
    expect(iconUrl("a", before)).toBeNull();

    emitMock("icons:updated", ["a"]);
    expect(version()).toBe(before + 1);

    resolveDir("C:\\cache");
    await ready;
    // Known folder: one more bump so icons drawn before it re-render.
    expect(version()).toBe(before + 2);
    expect(iconUrl("a", version())).toBe(`asset://localhost/C:\\cache\\a.png?v=${before + 2}`);
  });

  it("changes a rendered AppIcon src on a later icons:updated", async () => {
    invokeMock.mockResolvedValueOnce("C:\\cache");
    await initIconUrls();
    const { container } = render(<AppIcon appId="a" size={48} />);
    const src = () => container.querySelector("img")?.getAttribute("src");
    const first = src();
    expect(first).toContain(`?v=${version()}`);

    act(() => emitMock("icons:updated", ["a"]));
    expect(src()).toContain(`?v=${version()}`);
    expect(src()).not.toBe(first);
  });
});

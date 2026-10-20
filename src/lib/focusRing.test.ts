import { afterEach, describe, expect, it } from "vitest";
import { applyAlwaysShowFocus, quietFocusUntilKey } from "./focusRing";

const root = () => document.documentElement.dataset;
const press = (key = "Tab") => window.dispatchEvent(new KeyboardEvent("keydown", { key }));

afterEach(() => {
  delete root().focusQuiet;
  delete root().focusAlways;
});

describe("focus ring on open", () => {
  it("stays hidden after a show until a key is pressed", () => {
    quietFocusUntilKey(false);
    expect(root().focusQuiet).toBe("");
    expect(root().focusAlways).toBeUndefined();
    press("ArrowRight");
    expect(root().focusQuiet).toBeUndefined();
    // Later keys change nothing; the next show is quiet again.
    press();
    expect(root().focusQuiet).toBeUndefined();
    quietFocusUntilKey(false);
    expect(root().focusQuiet).toBe("");
  });

  it("shows at once with Always show keyboard focus", () => {
    quietFocusUntilKey(false);
    quietFocusUntilKey(true);
    expect(root().focusQuiet).toBeUndefined();
    expect(root().focusAlways).toBe("");
    applyAlwaysShowFocus(false);
    expect(root().focusAlways).toBeUndefined();
  });
});

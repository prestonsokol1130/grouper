import { describe, expect, it } from "vitest";
import {
  comboProblem,
  hotkeyParts,
  keyName,
  MESSAGES,
  recordPress,
  type KeyPress,
} from "./hotkeyCombo";

const press = (code: string, mods: Partial<KeyPress> = {}): KeyPress => ({
  code,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  ...mods,
});

describe("recordPress", () => {
  it("waits while only modifiers are down", () => {
    for (const code of ["ShiftLeft", "AltRight", "ControlLeft", "MetaLeft"]) {
      expect(recordPress(press(code, { altKey: true }))).toEqual({ kind: "waiting" });
    }
  });

  it("records the default and Shift+Alt+F12", () => {
    expect(recordPress(press("Backquote", { altKey: true }))).toEqual({
      kind: "combo",
      combo: { hotkey: "Alt+`", parts: ["Alt", "`"] },
    });
    expect(recordPress(press("F12", { shiftKey: true, altKey: true }))).toEqual({
      kind: "combo",
      combo: { hotkey: "Shift+Alt+F12", parts: ["Shift", "Alt", "F12"] },
    });
  });

  it("stores Win as Super and orders modifiers Ctrl, Shift, Alt, Win", () => {
    const recorded = recordPress(
      press("KeyG", { metaKey: true, ctrlKey: true, altKey: true, shiftKey: true }),
    );
    expect(recorded).toEqual({
      kind: "combo",
      combo: { hotkey: "Ctrl+Shift+Alt+Super+G", parts: ["Ctrl", "Shift", "Alt", "Win", "G"] },
    });
  });

  it("needs a modifier", () => {
    expect(recordPress(press("KeyA"))).toEqual({
      kind: "invalid",
      message: MESSAGES.needsModifier,
    });
    expect(recordPress(press("KeyA", { shiftKey: true }))).toMatchObject({ kind: "invalid" });
    expect(recordPress(press("F5", { shiftKey: true }))).toMatchObject({ kind: "combo" });
  });

  it("refuses shortcuts Windows uses", () => {
    for (const [code, mods] of [
      ["KeyL", { metaKey: true }],
      ["KeyD", { metaKey: true }],
      ["Tab", { altKey: true }],
      ["F4", { altKey: true }],
      ["Delete", { ctrlKey: true, altKey: true }],
      ["Escape", { ctrlKey: true, shiftKey: true }],
    ] as const) {
      expect(recordPress(press(code, mods))).toEqual({
        kind: "invalid",
        message: MESSAGES.reserved,
      });
    }
    expect(recordPress(press("KeyL", { metaKey: true, ctrlKey: true }))).toMatchObject({
      kind: "combo",
    });
  });

  it("refuses keys the shortcut can't use", () => {
    expect(recordPress(press("IntlBackslash", { altKey: true }))).toEqual({
      kind: "invalid",
      message: MESSAGES.unknownKey,
    });
  });
});

describe("helpers", () => {
  it("names keys the way the backend parses them", () => {
    expect(keyName("KeyQ")).toBe("Q");
    expect(keyName("Digit7")).toBe("7");
    expect(keyName("Numpad3")).toBe("Num3");
    expect(keyName("F24")).toBe("F24");
    expect(keyName("F25")).toBeNull();
    expect(keyName("ArrowLeft")).toBe("Left");
  });

  it("splits a stored shortcut into chips", () => {
    expect(hotkeyParts("Alt+`")).toEqual(["Alt", "`"]);
    expect(hotkeyParts("Shift+Super+F12")).toEqual(["Shift", "Win", "F12"]);
  });

  it("checks a combo on its own", () => {
    expect(comboProblem({ hotkey: "Ctrl+Alt+Space", parts: ["Ctrl", "Alt", "Space"] })).toBeNull();
    expect(comboProblem({ hotkey: "Alt+Space", parts: ["Alt", "Space"] })).toBe(MESSAGES.reserved);
  });
});

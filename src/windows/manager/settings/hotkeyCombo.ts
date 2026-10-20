/**
 * Turning a key press into a Launchpad shortcut, and the rules for one.
 * Pure. The rules match `src-tauri/src/hotkey_rules.rs`, which checks
 * again before registering.
 */

/** What the recorder needs from a keydown event. */
export type KeyPress = {
  readonly code: string;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
};

/** A recorded combination: the stored string and the chips to show. */
export type Combo = { readonly hotkey: string; readonly parts: readonly string[] };

/** The outcome of one key press while recording. */
export type Recorded =
  | { readonly kind: "waiting" }
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "combo"; readonly combo: Combo };

/** Codes of the modifier keys themselves: keep waiting for the real key. */
const MODIFIER_CODES = /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/;

/** Keys the shortcut plugin understands, by `KeyboardEvent.code`. */
const NAMED_KEYS: Readonly<Record<string, string>> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Space: "Space",
  Tab: "Tab",
  Enter: "Enter",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Escape: "Escape",
};

/** The stored name of the key with `code`, or `null` when it can't be used. */
export function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^(?:Digit|Numpad)([0-9])$/.exec(code);
  if (digit) return code.startsWith("Numpad") ? `Num${digit[1]}` : (digit[1] ?? null);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return NAMED_KEYS[code] ?? null;
}

/** Combinations Windows keeps for itself, as stored strings. */
const RESERVED = new Set(
  [
    ..."LDERIASXMVPKHNW".split("").map((key) => `Super+${key}`),
    "Super+Tab",
    "Super+Space",
    "Super+.",
    "Alt+Tab",
    "Alt+F4",
    "Alt+Escape",
    "Alt+Space",
    "Ctrl+Escape",
    "Ctrl+Shift+Escape",
    "Ctrl+Alt+Delete",
  ].map((hotkey) => hotkey.toUpperCase()),
);

/** DESIGN.md section 13 voice: what is wrong and what to do. */
export const MESSAGES = {
  needsModifier: "Use Ctrl, Alt or Win with another key.",
  reserved: "Windows uses that shortcut. Pick a different one.",
  unknownKey: "That key can't be used. Try a letter, number or F key.",
  taken: "That shortcut is in use by another app. Pick a different one.",
} as const;

/**
 * Why `combo` can't be the Launchpad shortcut, or `null` when it can. It
 * needs Ctrl, Alt or Win (Shift alone only with an F key, since Shift and
 * a letter is just typing), and not one Windows uses.
 */
export function comboProblem(combo: Combo): string | null {
  const [key = ""] = combo.parts.slice(-1);
  const mods = new Set(combo.parts.slice(0, -1));
  const strong = mods.has("Ctrl") || mods.has("Alt") || mods.has("Win");
  const shiftFKey = mods.has("Shift") && /^F\d+$/.test(key);
  if (!strong && !shiftFKey) return MESSAGES.needsModifier;
  if (RESERVED.has(combo.hotkey.toUpperCase())) return MESSAGES.reserved;
  return null;
}

/** Reads one key press while recording (see {@link Recorded}). */
export function recordPress(press: KeyPress): Recorded {
  if (MODIFIER_CODES.test(press.code)) return { kind: "waiting" };
  const key = keyName(press.code);
  if (key === null) return { kind: "invalid", message: MESSAGES.unknownKey };
  const mods: [boolean, string, string][] = [
    [press.ctrlKey, "Ctrl", "Ctrl"],
    [press.shiftKey, "Shift", "Shift"],
    [press.altKey, "Alt", "Alt"],
    [press.metaKey, "Super", "Win"],
  ];
  const held = mods.filter(([down]) => down);
  const combo: Combo = {
    hotkey: [...held.map(([, stored]) => stored), key].join("+"),
    parts: [...held.map(([, , shown]) => shown), key],
  };
  const problem = comboProblem(combo);
  return problem === null ? { kind: "combo", combo } : { kind: "invalid", message: problem };
}

/** The chips for a stored shortcut: `Shift+Super+F12` is Shift, Win, F12. */
export function hotkeyParts(hotkey: string): string[] {
  return hotkey
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part !== "")
    .map((part) => (part === "Super" ? "Win" : part));
}

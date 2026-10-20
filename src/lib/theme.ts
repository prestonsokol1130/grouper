import type { AccentColors, Material } from "../bindings";
import { api, type Result } from "./api";
import { onEvent } from "./events";

/** Light or dark, as Windows reports it. */
type Theme = "light" | "dark";

/** DESIGN.md section 3: the accent when the Windows one cannot be read. */
export const FALLBACK_ACCENT: AccentColors = { light: "#0067C0", dark: "#4CC2FF" };

const DARK_QUERY = "(prefers-color-scheme: dark)";

let theme: Theme = "light";
let accent: AccentColors = FALLBACK_ACCENT;
/** The "Accent color" setting when it is a custom colour. */
let custom: string | null = null;

/** Parses `#RRGGBB` into channels, or `null` when it is not one. */
export function parseHex(hex: string): [number, number, number] | null {
  const match = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!match) return null;
  const n = Number.parseInt(match[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** WCAG relative luminance of an sRGB colour. */
function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** The accent shades, DESIGN.md section 3. */
export type AccentShades = {
  accent: string;
  hover: string;
  pressed: string;
  onAccent: string;
};

/**
 * Derives the shades from `hex`: hover is 90% opacity, pressed 80%, and
 * text on the accent is black or white, whichever contrasts more.
 * `null` when `hex` is not `#RRGGBB`.
 */
export function accentShades(hex: string): AccentShades | null {
  const rgb = parseHex(hex);
  if (rgb === null) return null;
  const [r, g, b] = rgb;
  const l = luminance(rgb);
  const contrastWhite = 1.05 / (l + 0.05);
  const contrastBlack = (l + 0.05) / 0.05;
  return {
    accent: `rgb(${r}, ${g}, ${b})`,
    hover: `rgba(${r}, ${g}, ${b}, 0.9)`,
    pressed: `rgba(${r}, ${g}, ${b}, 0.8)`,
    onAccent: contrastBlack > contrastWhite ? "#000000" : "#FFFFFF",
  };
}

/** Writes the current theme and accent onto `<html>`. */
function paint(): void {
  const root = document.documentElement;
  root.dataset.theme = theme;
  const shades = accentShades(custom ?? accent[theme]) ?? accentShades(FALLBACK_ACCENT[theme]);
  if (shades === null) return;
  root.style.setProperty("--accent", shades.accent);
  root.style.setProperty("--accent-hover", shades.hover);
  root.style.setProperty("--accent-pressed", shades.pressed);
  root.style.setProperty("--text-on-accent", shades.onAccent);
}

/** Uses `colors` as the accent, or the fallback for `null`. */
export function setAccent(colors: AccentColors | null): void {
  accent = colors ?? FALLBACK_ACCENT;
  paint();
}

/**
 * Uses `hex` (`#RRGGBB`) as the accent in both themes instead of the
 * Windows one, or goes back to the Windows accent for `null` or anything
 * that is not a colour.
 */
export function setCustomAccent(hex: string | null): void {
  custom = hex !== null && parseHex(hex) !== null ? hex : null;
  paint();
}

/**
 * Follows the Windows light/dark setting: sets `data-theme` on `<html>`
 * now and on every change. Light/dark is not a Grouper setting. Returns a
 * function that stops following.
 */
export function applyTheme(): () => void {
  const media = window.matchMedia(DARK_QUERY);
  const update = () => {
    theme = media.matches ? "dark" : "light";
    paint();
  };
  update();
  media.addEventListener("change", update);
  return () => media.removeEventListener("change", update);
}

/** Reads the Windows accent and follows `accent:changed`. */
export function followAccent(): () => void {
  // A pushed accent is newer than the snapshot, which may resolve later.
  let pushed = false;
  const stop = onEvent("accent:changed", (colors) => {
    pushed = true;
    setAccent(colors);
  });
  void withRetry(api.getAccentColor).then((result) => {
    if (result.ok && !pushed) setAccent(result.value);
  });
  return stop;
}

/**
 * Runs `call` until it works, 15 times 200 ms apart at most: a page can
 * load before Rust has set up the accent and material state at startup.
 */
async function withRetry<T>(call: () => Promise<Result<T>>): Promise<Result<T>> {
  let result = await call();
  for (let attempt = 1; !result.ok && attempt < 15; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 200));
    result = await call();
  }
  return result;
}

/**
 * Marks `<html>` with the window's material; with `solid` the page paints
 * its own background (see styles.css). Outside Tauri it is solid.
 */
export async function applyMaterial(): Promise<Material> {
  const result = await withRetry(api.getWindowMaterial);
  const material: Material = result.ok ? result.value : "solid";
  document.documentElement.dataset.material = material;
  return material;
}

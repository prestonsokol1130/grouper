/**
 * When the keyboard focus ring shows (DESIGN.md section 11, changed per
 * Preston's feedback, 2026-10-20).
 *
 * The flyout and the Launchpad move keyboard focus into themselves when
 * they open, so the arrows and Enter work at once. Chromium shows the
 * ring for that programmatic focus, so a mouse or taskbar click used to
 * open with a ring around the first tile. Now, by default, the ring stays
 * hidden after a show until a key is pressed (`data-focus-quiet` on the
 * root, see components.css); after that `:focus-visible` decides as
 * usual. The "Always show keyboard focus" accessibility setting
 * (`data-focus-always`) shows the ring on every focused control,
 * including the one focused on open.
 */

const root = () => document.documentElement;

/** Clears the quiet mode on the first key press after a show. */
function onKey(): void {
  delete root().dataset.focusQuiet;
}

/**
 * Call on every show, before focusing the first control: hides the ring
 * until a key is pressed, unless `always` (the setting) is on.
 */
export function quietFocusUntilKey(always: boolean): void {
  applyAlwaysShowFocus(always);
  window.removeEventListener("keydown", onKey, true);
  if (always) {
    delete root().dataset.focusQuiet;
    return;
  }
  root().dataset.focusQuiet = "";
  window.addEventListener("keydown", onKey, { capture: true, once: true });
}

/** Follows the "Always show keyboard focus" setting. */
export function applyAlwaysShowFocus(always: boolean): void {
  if (always) root().dataset.focusAlways = "";
  else delete root().dataset.focusAlways;
}

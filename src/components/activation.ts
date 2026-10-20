import type { KeyboardEvent } from "react";

/** A state the style guide can force on a component without a pointer. */
export type PreviewState = "hover" | "pressed" | "focus" | "disabled";

/**
 * Key handlers that make Enter and Space activate a tile once. The
 * browser's own click on those keys is suppressed (Enter fires on
 * keydown, Space on keyup), so `activate` runs exactly once per press.
 */
export function activationKeys(activate: () => void) {
  return {
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        if (!event.repeat) activate();
      }
    },
    onKeyUp: (event: KeyboardEvent<HTMLElement>) => {
      if (event.key === " ") event.preventDefault();
    },
  };
}

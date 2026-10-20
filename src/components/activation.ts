import type { KeyboardEvent, Ref } from "react";

/** A state the style guide can force on a component without a pointer. */
export type PreviewState = "hover" | "pressed" | "focus" | "disabled";

/**
 * Extra props for a tile's button, for a drag-and-drop wrapper: its ref
 * (the keyboard drag handle) and the drag's screen-reader description.
 */
export type TileButtonProps = {
  ref?: Ref<HTMLButtonElement>;
  "aria-roledescription"?: string;
  "aria-describedby"?: string;
};

/**
 * Key handlers that make Enter and Space activate a tile once. The
 * browser's own click on those keys is suppressed (Enter fires on
 * keydown, Space on keyup), so `activate` runs exactly once per press.
 * A key something else already claimed (a drag that started or ended on
 * it) is left alone.
 */
export function activationKeys(activate: () => void) {
  return {
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (event.defaultPrevented) return;
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

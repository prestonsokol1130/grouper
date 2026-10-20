import { useEffect, useRef, type RefObject } from "react";

type Options = {
  query: string;
  setQuery: (query: string) => void;
  inputRef: RefObject<HTMLInputElement | null>;
  close: () => void;
  /** Turns the page by `step` (no search active only). */
  turn: (step: number) => void;
  /** True while a tile is dragged: the drag owns the keys. */
  paused?: boolean;
};

/** Keys that page while no search is active. */
const PAGE_KEYS: Readonly<Record<string, number>> = {
  ArrowLeft: -1,
  PageUp: -1,
  ArrowRight: 1,
  PageDown: 1,
};

/** A key that types a character: one character, no Ctrl, Alt or Win. */
function typesText(event: KeyboardEvent): boolean {
  return event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey;
}

/**
 * The Launchpad's window-wide keys:
 * - Escape clears the search first, and closes the Launchpad only when
 *   the search is already empty.
 * - Typing anywhere outside the search box (and Backspace) goes into it.
 * - Left/Right and PageUp/PageDown turn pages, but only with no search:
 *   during a search the arrows move between results.
 * Keys a tile already handled (Enter, Space) are left alone, and so is
 * every key while a tile is dragged.
 */
export function useLaunchpadKeys({
  query,
  setQuery,
  inputRef,
  close,
  turn,
  paused = false,
}: Options): void {
  const latest = useRef({ query, setQuery, close, turn, paused });
  useEffect(() => {
    latest.current = { query, setQuery, close, turn, paused };
  });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || latest.current.paused) return;
      const { query: current, setQuery: set, close: hide, turn: page } = latest.current;
      const input = inputRef.current;
      const inSearch = input !== null && event.target === input;
      if (event.key === "Escape") {
        event.preventDefault();
        if (current === "") hide();
        else {
          set("");
          input?.focus();
        }
        return;
      }
      if (!inSearch && input !== null && (typesText(event) || event.key === "Backspace")) {
        event.preventDefault();
        set(event.key === "Backspace" ? current.slice(0, -1) : current + event.key);
        input.focus();
        return;
      }
      const step = PAGE_KEYS[event.key];
      if (step !== undefined && current === "") {
        event.preventDefault();
        page(step);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [inputRef]);
}

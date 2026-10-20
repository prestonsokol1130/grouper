import { useLayoutEffect, type KeyboardEvent, type RefObject } from "react";
import { FluentIcon } from "../../components/FluentIcon";

type Props = {
  value: string;
  onChange: (value: string) => void;
  /** Changes on every `launchpad:show`: the box is focused and selected. */
  showSeq: number;
  inputRef: RefObject<HTMLInputElement | null>;
  /** Enter: launch the first result. */
  onSubmit: () => void;
  /** Down arrow: move focus into the results. */
  onFocusResults: () => void;
};

/**
 * The Launchpad search pill (DESIGN.md section 8): centred, 320 x 36px,
 * radius 18px, the Fluent TextBox look with the search glyph. Focused,
 * with its text selected, every time the Launchpad shows. Escape and
 * typing elsewhere are handled by the Launchpad (`useLaunchpadKeys`).
 */
export function SearchBar({ value, onChange, showSeq, inputRef, onSubmit, onFocusResults }: Props) {
  useLayoutEffect(() => {
    if (showSeq === 0) return;
    const input = inputRef.current;
    input?.focus({ preventScroll: true });
    input?.select();
  }, [showSeq, inputRef]);

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      if (!event.repeat) onSubmit();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      onFocusResults();
    }
  };

  return (
    <div className="launchpad-search relative mx-auto w-[320px] max-w-full shrink-0">
      <span className="pointer-events-none absolute top-1/2 left-3.5 -translate-y-1/2 text-secondary">
        <FluentIcon glyph="search" />
      </span>
      <input
        ref={inputRef}
        type="search"
        value={value}
        aria-label="Search apps"
        placeholder="Search apps"
        autoComplete="off"
        spellCheck={false}
        className="text-box h-9 w-full rounded-[18px] pr-4 pl-10 type-body text-primary"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
    </div>
  );
}

import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { checkGroupName } from "./groupName";

type Props = {
  /** The current name, shown selected so typing replaces it. */
  initial: string;
  /** Called with a valid, trimmed name that differs from `initial`. */
  onSave: (name: string) => void;
  /** Called when editing ends, saved or not. */
  onDone: () => void;
};

/**
 * The inline rename box over a group's page title. Enter saves, Escape
 * cancels, and an empty name is refused with a message (the box stays
 * open). Leaving the box saves a valid name and drops an empty one.
 */
export function RenameField({ initial, onSave, onDone }: Props) {
  const [value, setValue] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  // Enter, Escape and blur can all end editing; only the first counts.
  const ended = useRef(false);
  const errorId = useId();

  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);

  const end = (save: boolean, invalidKeepsOpen: boolean) => {
    if (ended.current) return;
    const check = checkGroupName(value);
    if (save && !check.ok) {
      if (invalidKeepsOpen) {
        setError(check.message);
        return;
      }
      save = false;
    }
    ended.current = true;
    if (save && check.ok && check.name !== initial) onSave(check.name);
    onDone();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      end(true, true);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      end(false, false);
    }
  };

  return (
    <div className="min-w-0 flex-1">
      <input
        ref={input}
        value={value}
        aria-label="Group name"
        aria-invalid={error !== null}
        aria-describedby={error !== null ? errorId : undefined}
        className="text-box h-11 w-full max-w-[480px] rounded-control px-2 type-title text-primary"
        onChange={(event) => {
          setValue(event.target.value);
          if (error !== null && checkGroupName(event.target.value).ok) setError(null);
        }}
        onKeyDown={onKeyDown}
        onBlur={() => end(true, false)}
      />
      {error !== null && (
        <p id={errorId} role="alert" className="mt-1 type-caption text-danger">
          {error}
        </p>
      )}
    </div>
  );
}

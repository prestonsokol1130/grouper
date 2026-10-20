import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Button } from "../../../components/Button";
import { hotkeyParts, MESSAGES, recordPress } from "./hotkeyCombo";
import { saveHotkey } from "./settingsActions";
import { SettingCard } from "./SettingCard";

/** The keys of a shortcut as Fluent keyboard-accelerator chips. */
export function KeyChips({ parts, label }: { parts: readonly string[]; label: string }) {
  return (
    <span className="flex items-center gap-1" aria-label={label} role="img">
      {parts.map((part, index) => (
        <kbd key={`${part}-${index}`} className="key-chip type-body-strong" aria-hidden="true">
          {part}
        </kbd>
      ))}
    </span>
  );
}

type State =
  | { kind: "idle" }
  | { kind: "recording"; message: string | null }
  | { kind: "saving"; parts: readonly string[] };

/**
 * The "Launchpad shortcut" card. Change starts recording: the next key
 * press with its modifiers is checked ({@link recordPress}) and, if it is
 * allowed, registered with set_hotkey. Escape cancels. If Windows says
 * another app holds it, the old shortcut stays and the card says so.
 */
export function HotkeyRecorder({ hotkey }: { hotkey: string }) {
  const [state, setState] = useState<State>({ kind: "idle" });
  const [error, setError] = useState<string | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const recording = state.kind === "recording";
  const parts = state.kind === "saving" ? state.parts : hotkeyParts(hotkey);

  useEffect(() => {
    if (recording) box.current?.focus();
  }, [recording]);

  const start = () => {
    setError(null);
    setState({ kind: "recording", message: null });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    const plain = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (event.key === "Escape" && plain) {
      setState({ kind: "idle" });
      return;
    }
    const recorded = recordPress(event);
    if (recorded.kind === "waiting") return;
    if (recorded.kind === "invalid") {
      setState({ kind: "recording", message: recorded.message });
      return;
    }
    const { combo } = recorded;
    setState({ kind: "saving", parts: combo.parts });
    void saveHotkey(combo.hotkey).then((ok) => {
      setState({ kind: "idle" });
      if (!ok) setError(MESSAGES.taken);
    });
  };

  const message = state.kind === "recording" ? state.message : error;
  return (
    <SettingCard
      title="Launchpad shortcut"
      description="Opens the Launchpad from anywhere."
      footer={
        message && (
          <p role="alert" className="mt-2 type-caption text-danger">
            {message}
          </p>
        )
      }
    >
      {recording ? (
        <div
          ref={box}
          tabIndex={0}
          role="textbox"
          aria-label="Press the new shortcut"
          className="text-box flex h-8 min-w-[180px] items-center rounded-control px-3 type-body text-secondary"
          onKeyDown={onKeyDown}
          onBlur={() => setState({ kind: "idle" })}
        >
          Press the new shortcut…
        </div>
      ) : (
        <KeyChips parts={parts} label={`Shortcut: ${parts.join(" + ")}`} />
      )}
      {recording ? (
        <Button
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => setState({ kind: "idle" })}
        >
          Cancel
        </Button>
      ) : (
        <Button onClick={start} disabled={state.kind === "saving"}>
          {state.kind === "saving" ? "Saving…" : "Change"}
        </Button>
      )}
    </SettingCard>
  );
}

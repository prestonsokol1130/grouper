import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Button } from "../../components/Button";
import { useAppStore } from "../../store/appStore";
import { groupsById, visibleApps } from "../../store/selectors";
import { applySelection, pickerApps, toggleId } from "./appSearch";
import { FluentIcon } from "./FluentIcon";
import { pickOtherPrograms, rescanStartMenu } from "./groupActions";
import { PickerList } from "./PickerList";

type Props = { groupId: string; groupName: string; onClose: () => void };

/** The group's current app ids. */
function currentIds(groupId: string): string[] {
  const state = useAppStore.getState().state;
  return (state ? groupsById(state).get(groupId)?.appIds : undefined) ?? [];
}

/** Keeps Tab inside `panel`, like ConfirmDialog. */
function trapTab(event: KeyboardEvent<HTMLElement>, panel: HTMLElement | null): void {
  const selector = "input:not(:disabled), button:not(:disabled)";
  const items = [...(panel?.querySelectorAll<HTMLElement>(selector) ?? [])];
  const first = items[0];
  const last = items[items.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last?.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first?.focus();
  }
}

/**
 * The "Add apps" dialog: a search box (name or initials, so "vsc" finds
 * Visual Studio Code), every visible app with a checkbox (the group's apps
 * start ticked), Rescan Start Menu and Add other program. Done saves the
 * ticks with set_group_apps: kept apps stay in order and new ones go at
 * the end. Escape or Cancel closes without saving.
 */
export function AppPicker({ groupId, groupName, onClose }: Props) {
  const state = useAppStore((store) => store.state);
  const setGroupApps = useAppStore((store) => store.setGroupApps);
  const apps = useMemo(() => (state ? visibleApps(state) : []), [state]);
  const [query, setQuery] = useState("");
  const [selection, setSelection] = useState<string[]>(() => {
    const listed = new Set(apps.map((app) => app.id));
    return currentIds(groupId).filter((id) => listed.has(id));
  });
  const [scanning, setScanning] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const shown = useMemo(() => pickerApps(apps, query), [apps, query]);
  const selected = useMemo(() => new Set(selection), [selection]);

  useEffect(() => {
    const previous = document.activeElement;
    panel.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    return () => {
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);

  const done = () => {
    const before = currentIds(groupId);
    const listed = new Set(apps.flatMap((app) => (app.id === undefined ? [] : [app.id])));
    const next = applySelection(before, listed, selection);
    if (next.join("\n") !== before.join("\n")) void setGroupApps(groupId, next);
    onClose();
  };

  const rescan = () => {
    setScanning(true);
    void rescanStartMenu().finally(() => setScanning(false));
  };

  const addOther = () => {
    void pickOtherPrograms().then((ids) =>
      setSelection((current) => [...current, ...ids.filter((id) => !current.includes(id))]),
    );
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (event.key === "Tab") {
      trapTab(event, panel.current);
    }
  };

  const onSearchKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Escape" || query === "") return;
    event.preventDefault();
    event.stopPropagation();
    setQuery("");
  };

  return (
    <div className="dialog-smoke fixed inset-0 z-40 flex items-center justify-center p-4">
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="flex h-[min(640px,100%)] w-[480px] max-w-full flex-col overflow-hidden rounded-window border border-stroke-card bg-layer text-primary shadow-raised"
        onKeyDown={onKeyDown}
      >
        <div className="flex flex-col gap-3 px-6 pt-6 pb-3">
          <h2 id={titleId} className="truncate type-subtitle" title={`Add apps to ${groupName}`}>
            Add apps to {groupName}
          </h2>
          <div className="relative">
            <span className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-secondary">
              <FluentIcon glyph="search" />
            </span>
            <input
              type="search"
              value={query}
              aria-label="Search apps"
              placeholder="Search apps"
              data-autofocus=""
              className="text-box h-8 w-full rounded-control pr-2 pl-9 type-body text-primary"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={onSearchKey}
            />
          </div>
          <div className="flex items-center justify-between gap-2">
            <p className="type-caption text-secondary" aria-live="polite">
              {scanning ? "Scanning the Start Menu…" : `${selection.length} selected`}
            </p>
            <Button variant="ghost" onClick={rescan} disabled={scanning}>
              {scanning ? (
                <span aria-hidden="true" className="progress-ring" />
              ) : (
                <FluentIcon glyph="refresh" />
              )}
              {scanning ? "Scanning…" : "Rescan Start Menu"}
            </Button>
          </div>
        </div>
        {shown.length > 0 ? (
          <PickerList
            key={query}
            apps={shown}
            selected={selected}
            onToggle={(id) => setSelection((current) => toggleId(current, id))}
          />
        ) : (
          <p className="flex-1 px-6 py-2 type-body text-secondary">
            {apps.length === 0
              ? "No apps found yet. Click Rescan Start Menu."
              : `No apps match "${query.trim()}". Try another name.`}
          </p>
        )}
        <div className="flex items-center gap-2 border-t border-stroke-card bg-surface px-6 py-4">
          <Button onClick={addOther}>
            <FluentIcon glyph="add" />
            Add other program…
          </Button>
          <div className="flex-1" />
          <Button variant="primary" onClick={done}>
            Done
          </Button>
          <Button onClick={onClose}>Cancel</Button>
        </div>
      </div>
    </div>
  );
}

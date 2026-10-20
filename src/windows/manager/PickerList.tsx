import type { KeyboardEvent } from "react";
import type { AppEntry } from "../../bindings";
import { AppIcon } from "../../components/AppIcon";
import { FluentIcon } from "./FluentIcon";
import { useWindowedRows } from "./useWindowedRows";

/** Fluent list item height (px); fixed, so long lists can be windowed. */
const ROW_PX = 40;
/** Icon size in a picker row: Fluent's 24px list icon. */
const ICON_PX = 24;

type Props = {
  apps: AppEntry[];
  selected: ReadonlySet<string>;
  onToggle: (appId: string) => void;
};

/**
 * The picker's app list: one checkbox row per app, with its icon and
 * name. Up and Down move between rows, Space ticks. Over 300 apps only
 * the rows near the viewport are rendered.
 */
export function PickerList({ apps, selected, onToggle }: Props) {
  const { viewport, windowed, range, totalHeight, reveal, onScroll } = useWindowedRows(
    apps.length,
    ROW_PX,
  );
  const { start, end } = range;

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>, index: number) => {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = index + step;
    if (next < 0 || next >= apps.length) return;
    reveal(next);
    requestAnimationFrame(() =>
      viewport.current?.querySelector<HTMLElement>(`input[data-index="${next}"]`)?.focus(),
    );
  };

  return (
    <div ref={viewport} onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto px-4">
      <ul
        aria-label="Apps"
        className="relative"
        style={windowed ? { height: totalHeight } : undefined}
      >
        {apps.slice(start, end).map((app, offset) => {
          const index = start + offset;
          const id = app.id ?? "";
          const name = app.name ?? "";
          return (
            <li
              key={id}
              aria-setsize={apps.length}
              aria-posinset={index + 1}
              className={windowed ? "absolute right-0 left-0" : undefined}
              style={windowed ? { top: index * ROW_PX } : undefined}
            >
              <label
                className="picker-row subtle relative flex items-center gap-3 rounded-control px-2"
                style={{ height: ROW_PX }}
                title={name}
              >
                <input
                  type="checkbox"
                  className="picker-check sr-only"
                  checked={selected.has(id)}
                  data-index={index}
                  onChange={() => onToggle(id)}
                  onKeyDown={(event) => onKeyDown(event, index)}
                />
                <span aria-hidden="true" className="check-box">
                  <FluentIcon glyph="check" />
                </span>
                <span className="block shrink-0" data-missing={app.missing === true}>
                  <AppIcon appId={id} size={ICON_PX} />
                </span>
                <span className="min-w-0 flex-1 truncate type-body">{name}</span>
                {app.missing === true && (
                  <span className="shrink-0 type-caption text-secondary">Can't be found</span>
                )}
              </label>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

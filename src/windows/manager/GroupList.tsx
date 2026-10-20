import { useRef, type KeyboardEvent } from "react";
import { Button } from "../../components/Button";
import { FolderArt } from "../../components/FolderTile";
import { useGroup } from "../../store/hooks";
import { FluentIcon } from "./FluentIcon";
import { moveItem, useDragReorder } from "./useDragReorder";

/** Folder preview size in a sidebar row (px). */
const PREVIEW_PX = 32;

export type Page = { kind: "group"; id: string } | { kind: "settings" };

type Props = {
  ids: string[];
  page: Page | null;
  onSelect: (page: Page) => void;
  onNewGroup: () => void;
  onRename: (id: string) => void;
  onReorder: (ids: string[]) => void;
};

const ROW =
  "nav-item focus-ring relative flex w-full items-center gap-3 rounded-control px-3 text-left";

/** One group in the sidebar: its live folder preview and its name. */
function GroupRow({ id }: { id: string }) {
  const group = useGroup(id);
  const name = group?.name ?? "";
  return (
    <>
      <FolderArt groupId={id} size={PREVIEW_PX} />
      <span className="min-w-0 flex-1 truncate type-body" title={name}>
        {name}
      </span>
    </>
  );
}

/**
 * The Manager sidebar (DESIGN.md section 8): a New group button, the
 * groups with live folder previews, and Settings at the bottom. Rows are
 * reordered by dragging or with Alt+Up / Alt+Down; Up and Down move
 * between rows and F2 renames.
 */
export function GroupList({ ids, page, onSelect, onNewGroup, onRename, onReorder }: Props) {
  const list = useRef<HTMLUListElement>(null);
  const drag = useDragReorder(ids, onReorder);
  const selectedId = page?.kind === "group" ? page.id : null;

  const focusRow = (id: string) =>
    requestAnimationFrame(() =>
      list.current?.querySelector<HTMLElement>(`[data-group-id="${id}"]`)?.focus(),
    );

  const onRowKey = (event: KeyboardEvent<HTMLButtonElement>, id: string) => {
    const index = ids.indexOf(id);
    const step = event.key === "ArrowUp" ? -1 : event.key === "ArrowDown" ? 1 : 0;
    if (event.key === "Escape") drag.cancel();
    if (event.key === "F2") {
      event.preventDefault();
      onRename(id);
      return;
    }
    if (step === 0 || index < 0) return;
    event.preventDefault();
    const to = index + step;
    if (to < 0 || to >= ids.length) return;
    if (event.altKey) {
      onReorder(moveItem(ids, index, to));
      focusRow(id);
    } else {
      const next = ids[to];
      if (next !== undefined) focusRow(next);
    }
  };

  return (
    <aside className="flex w-[280px] shrink-0 flex-col gap-2 px-2 pt-3 pb-2">
      <div className="px-1">
        <Button onClick={onNewGroup}>
          <FluentIcon glyph="add" />
          New group
        </Button>
      </div>
      <nav aria-label="Groups" className="min-h-0 flex-1 overflow-y-auto">
        <ul
          ref={list}
          className="flex flex-col gap-1 p-1"
          data-reordering={drag.draggingId !== null}
        >
          {ids.map((id) => {
            const dragging = drag.draggingId === id;
            const shift = drag.shift(id);
            return (
              <li key={id}>
                <button
                  type="button"
                  data-group-id={id}
                  data-dragging={dragging}
                  aria-current={selectedId === id ? "page" : undefined}
                  aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown F2"
                  className={`${ROW} h-11 touch-none select-none`}
                  style={shift !== 0 ? { translate: `0 ${shift}px` } : undefined}
                  onClick={() => onSelect({ kind: "group", id })}
                  onKeyDown={(event) => onRowKey(event, id)}
                  {...drag.rowProps(id)}
                >
                  <GroupRow id={id} />
                </button>
              </li>
            );
          })}
        </ul>
      </nav>
      <div className="px-1">
        <button
          type="button"
          aria-current={page?.kind === "settings" ? "page" : undefined}
          className={`${ROW} h-9`}
          onClick={() => onSelect({ kind: "settings" })}
        >
          <FluentIcon glyph="settings" />
          <span className="type-body">Settings</span>
        </button>
      </div>
    </aside>
  );
}

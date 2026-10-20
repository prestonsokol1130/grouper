import { useMemo, useState } from "react";
import { useAppStore } from "../../store/appStore";
import { createGroup } from "./groupActions";
import { GroupDetail } from "./GroupDetail";
import { GroupList, type Page } from "./GroupList";
import { NoGroups, SettingsPlaceholder } from "./ManagerPages";

/**
 * The page to show: Settings when chosen, else the chosen group while it
 * exists, else the first group (so a deleted group's page moves on), else
 * nothing.
 */
function resolvePage(chosen: Page | null, ids: string[]): Page | null {
  if (chosen?.kind === "settings") return chosen;
  if (chosen !== null && ids.includes(chosen.id)) return chosen;
  const first = ids[0];
  return first === undefined ? null : { kind: "group", id: first };
}

/**
 * The Manager window (DESIGN.md section 8): the group sidebar on the left
 * and the selected page on the right. The root stays transparent so the
 * window's Mica shows through; the content pane is the `--layer-bg` layer
 * with its top-left corner rounded, like WinUI's NavigationView.
 */
export function ManagerRoot() {
  const groups = useAppStore((store) => store.state?.groups);
  const loaded = useAppStore((store) => store.state !== null);
  const reorderGroups = useAppStore((store) => store.reorderGroups);
  const ids = useMemo(() => (groups ?? []).map((group) => group.id ?? ""), [groups]);
  const [chosen, setChosen] = useState<Page | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const page = resolvePage(chosen, ids);

  const select = (next: Page) => {
    setChosen(next);
    setRenaming(null);
  };

  const newGroup = () => {
    void createGroup().then((id) => {
      if (id === null) return;
      // The group arrives with the backend's state:changed; until then the
      // page falls back to the first group, then switches to it.
      setChosen({ kind: "group", id });
      setRenaming(id);
    });
  };

  const rename = (id: string) => {
    setChosen({ kind: "group", id });
    setRenaming(id);
  };

  return (
    <div className="flex h-screen text-primary">
      <GroupList
        ids={ids}
        page={page}
        onSelect={select}
        onNewGroup={newGroup}
        onRename={rename}
        onReorder={(next) => {
          // Pin the page shown, so moving the first group doesn't change it.
          if (chosen === null && page !== null) setChosen(page);
          void reorderGroups(next);
        }}
      />
      <main className="flex min-w-0 flex-1 flex-col overflow-y-auto rounded-tl-window border-t border-l border-stroke-card bg-layer px-9 py-7">
        {loaded && page === null && <NoGroups onNewGroup={newGroup} />}
        {page?.kind === "settings" && <SettingsPlaceholder />}
        {page?.kind === "group" && (
          <GroupDetail
            key={page.id}
            groupId={page.id}
            renaming={renaming === page.id}
            onRenameStart={() => setRenaming(page.id)}
            onRenameEnd={() => setRenaming(null)}
          />
        )}
      </main>
    </div>
  );
}

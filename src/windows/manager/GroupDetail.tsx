import { useMemo, useState } from "react";
import { Button } from "../../components/Button";
import { useAppStore } from "../../store/appStore";
import { useGroup } from "../../store/hooks";
import { groupApps } from "../../store/selectors";
import { AppGrid } from "./AppGrid";
import { AppPicker } from "./AppPicker";
import { withShownOrder } from "./appSearch";
import { ConfirmDialog } from "./ConfirmDialog";
import { FluentIcon } from "../../components/FluentIcon";
import { deleteGroup, openApp } from "./groupActions";
import { RenameField } from "./RenameField";
import { TaskbarSection } from "./TaskbarSection";

/** DESIGN.md section 13. */
const EMPTY_MESSAGE = "No apps in this group yet. Drag apps here or click Add apps.";

type Props = {
  groupId: string;
  renaming: boolean;
  onRenameStart: () => void;
  onRenameEnd: () => void;
};

/** "1 app", "3 apps". */
function appCount(count: number): string {
  return count === 1 ? "1 app" : `${count} apps`;
}

/**
 * The selected group's page: its name as the page title (renamed inline),
 * Rename and Delete, its apps as tiles (Add apps, drag to reorder, Remove)
 * and the Taskbar card with the live icon and the shortcut actions.
 */
export function GroupDetail({ groupId, renaming, onRenameStart, onRenameEnd }: Props) {
  const group = useGroup(groupId);
  const state = useAppStore((store) => store.state);
  const renameGroup = useAppStore((store) => store.renameGroup);
  const setGroupApps = useAppStore((store) => store.setGroupApps);
  const apps = useMemo(() => (state ? groupApps(state, groupId) : []), [state, groupId]);
  const [confirming, setConfirming] = useState(false);
  const [picking, setPicking] = useState(false);
  if (group === undefined) return null;
  const name = group.name ?? "";
  const allIds = group.appIds ?? [];
  const shownIds = apps.map((app) => app.id ?? "");

  const confirmDelete = () => {
    setConfirming(false);
    void deleteGroup(groupId);
  };

  const reorder = (ids: string[]) =>
    void setGroupApps(groupId, withShownOrder(allIds, shownIds, ids));
  const remove = (appId: string) =>
    void setGroupApps(
      groupId,
      allIds.filter((id) => id !== appId),
    );
  const open = (appId: string) =>
    void openApp(appId, apps.find((app) => app.id === appId)?.name ?? "", () => remove(appId));

  return (
    <section aria-label={name} className="flex flex-col">
      <header className="flex items-start gap-3">
        {renaming ? (
          <RenameField
            key={groupId}
            initial={name}
            onSave={(next) => void renameGroup(groupId, next)}
            onDone={onRenameEnd}
          />
        ) : (
          <h1
            className="min-w-0 flex-1 truncate py-1 type-title"
            title={name}
            onDoubleClick={onRenameStart}
          >
            {name}
          </h1>
        )}
        <div className="flex shrink-0 gap-2 pt-2">
          <Button onClick={onRenameStart} disabled={renaming}>
            Rename
          </Button>
          <Button onClick={() => setConfirming(true)}>Delete</Button>
        </div>
      </header>
      <p className="mt-1 mb-6 type-body text-secondary">{appCount(apps.length)}</p>
      <div className="mb-3 flex items-center gap-2">
        <h2 className="flex-1 type-subtitle">Apps</h2>
        <Button variant="primary" onClick={() => setPicking(true)}>
          <FluentIcon glyph="add" />
          Add apps
        </Button>
      </div>
      {apps.length === 0 ? (
        <p className="type-body text-secondary">{EMPTY_MESSAGE}</p>
      ) : (
        <AppGrid groupName={name} apps={apps} onOpen={open} onReorder={reorder} onRemove={remove} />
      )}
      <TaskbarSection groupId={groupId} />
      {picking && (
        <AppPicker groupId={groupId} groupName={name} onClose={() => setPicking(false)} />
      )}
      {confirming && (
        <ConfirmDialog
          title={`Delete ${name}?`}
          confirmLabel="Delete"
          onConfirm={confirmDelete}
          onCancel={() => setConfirming(false)}
        >
          <p>Its taskbar shortcut will be removed too. Your apps stay on your PC.</p>
        </ConfirmDialog>
      )}
    </section>
  );
}

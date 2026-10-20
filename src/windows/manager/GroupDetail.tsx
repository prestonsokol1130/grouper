import { useMemo, useState } from "react";
import { AppTile } from "../../components/AppTile";
import { Button } from "../../components/Button";
import { useAppStore } from "../../store/appStore";
import { useGroup } from "../../store/hooks";
import { groupApps } from "../../store/selectors";
import { ConfirmDialog } from "./ConfirmDialog";
import { deleteGroup } from "./groupActions";
import { RenameField } from "./RenameField";

/** App icon size in the Manager grid: the Launchpad medium (DESIGN.md section 7). */
const TILE_ICON_PX = 72;

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
 * Rename and Delete, and its apps as a read-only grid of tiles. Adding and
 * removing apps comes with the app picker (S4-06).
 */
export function GroupDetail({ groupId, renaming, onRenameStart, onRenameEnd }: Props) {
  const group = useGroup(groupId);
  const state = useAppStore((store) => store.state);
  const renameGroup = useAppStore((store) => store.renameGroup);
  const apps = useMemo(() => (state ? groupApps(state, groupId) : []), [state, groupId]);
  const [confirming, setConfirming] = useState(false);
  if (group === undefined) return null;
  const name = group.name ?? "";

  const confirmDelete = () => {
    setConfirming(false);
    void deleteGroup(groupId);
  };

  return (
    <section aria-label={name} className="flex min-h-0 flex-1 flex-col">
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
      {apps.length === 0 ? (
        <p className="type-body text-secondary">No apps in this group yet.</p>
      ) : (
        <ul aria-label={`Apps in ${name}`} className="flex flex-wrap content-start gap-2">
          {apps.map((app) => (
            <li key={app.id}>
              <AppTile appId={app.id ?? ""} size={TILE_ICON_PX} />
            </li>
          ))}
        </ul>
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

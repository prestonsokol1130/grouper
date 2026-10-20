import { useState } from "react";
import { Button } from "../../components/Button";
import { FolderArt } from "../../components/FolderTile";
import { useGroup } from "../../store/hooks";
import { FluentIcon } from "../../components/FluentIcon";
import { createShortcut, revealShortcut } from "./groupActions";
import { PinSteps } from "./PinSteps";

/** The live taskbar icon preview, the Launchpad medium icon size. */
const PREVIEW_PX = 72;

/**
 * The group's taskbar card: a live preview of its 2x2 folder icon, then
 * either "Create taskbar shortcut" or, once the shortcut exists, "Show in
 * Explorer" and the one manual step Windows needs to pin it. Without
 * `heading` its title is kept for screen readers only (the welcome).
 */
export function TaskbarSection({
  groupId,
  heading = true,
}: {
  groupId: string;
  heading?: boolean;
}) {
  const group = useGroup(groupId);
  const [creating, setCreating] = useState(false);
  const name = group?.name ?? "";
  const hasShortcut = group?.hasShortcut === true;

  const create = () => {
    setCreating(true);
    void createShortcut(groupId).finally(() => setCreating(false));
  };

  return (
    <section aria-labelledby={`taskbar-${groupId}`} className={heading ? "mt-8" : ""}>
      <h2 id={`taskbar-${groupId}`} className={heading ? "mb-3 type-subtitle" : "sr-only"}>
        Taskbar
      </h2>
      <div className="flex items-start gap-4 rounded-window border border-stroke-card bg-card p-4">
        <figure className="flex shrink-0 flex-col items-center gap-1">
          <FolderArt groupId={groupId} size={PREVIEW_PX} />
          <figcaption className="type-caption text-secondary">Icon preview</figcaption>
        </figure>
        <div className="min-w-0 flex-1">
          {hasShortcut ? (
            <>
              <p className="type-body-strong">The shortcut is ready.</p>
              <p className="mb-3 type-body text-secondary">
                The icon shows the group's first four apps and updates when they change.
              </p>
              <Button onClick={() => void revealShortcut(groupId)}>
                <FluentIcon glyph="folderOpen" />
                Show in Explorer
              </Button>
              <PinSteps name={name} />
            </>
          ) : (
            <>
              <p className="type-body-strong">Put this group on your taskbar.</p>
              <p className="mb-3 type-body text-secondary">
                Grouper makes a shortcut with this icon. You pin it to the taskbar in one step.
              </p>
              <Button variant="primary" onClick={create} disabled={creating}>
                {creating ? "Creating shortcut…" : "Create taskbar shortcut"}
              </Button>
            </>
          )}
        </div>
      </div>
    </section>
  );
}

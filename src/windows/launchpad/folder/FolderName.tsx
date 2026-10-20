import { useEffect, useRef, useState } from "react";
import { useAppStore } from "../../../store/appStore";
import { RenameField } from "../../manager/RenameField";

type Props = {
  groupId: string;
  name: string;
};

/**
 * The folder's name at the top of the panel, in Subtitle (DESIGN.md
 * section 6, as the flyout's group name). Clicking it, or Enter on it,
 * edits it in place with the Manager's RenameField: Enter saves through
 * rename_group, Escape cancels, an empty name is refused. Focus goes back
 * to the name when editing ends.
 */
export function FolderName({ groupId, name }: Props) {
  const [editing, setEditing] = useState(false);
  const rename = useAppStore((store) => store.renameGroup);
  const button = useRef<HTMLButtonElement>(null);
  const edited = useRef(false);

  useEffect(() => {
    if (editing || !edited.current) return;
    edited.current = false;
    // Enter or Escape left focus nowhere; a click elsewhere keeps its own.
    const active = document.activeElement;
    if (active === null || active === document.body) button.current?.focus();
  }, [editing]);

  if (editing) {
    return (
      <div className="flex min-w-0 flex-1">
        <RenameField
          initial={name}
          variant="subtitle"
          onSave={(next) => void rename(groupId, next)}
          onDone={() => {
            edited.current = true;
            setEditing(false);
          }}
        />
      </div>
    );
  }
  return (
    <h2 className="flex min-w-0 flex-1">
      <button
        ref={button}
        type="button"
        className="subtle focus-ring h-9 max-w-full truncate rounded-control px-2 text-left type-subtitle"
        title="Click to rename"
        onClick={() => setEditing(true)}
      >
        {name}
      </button>
    </h2>
  );
}

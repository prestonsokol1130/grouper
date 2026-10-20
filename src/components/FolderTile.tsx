import { useFolderIconApps, useGroup } from "../store/hooks";
import { activationKeys, type PreviewState } from "./activation";
import { AppIcon } from "./AppIcon";
import { miniLayout, shapeStyle, type FolderShape } from "./folderLayout";

type Props = {
  groupId: string;
  onOpen?: (groupId: string) => void;
  /** Tile side in px; matches the app icon size around it. */
  size?: number;
  /** The "Folder shape" setting; Windows square by default. */
  shape?: FolderShape;
  showLabel?: boolean;
  /** Style guide only: shows a state without a pointer or keyboard. */
  preview?: PreviewState;
};

/** Space around the tile art inside the button, as in AppTile. */
const INSET = 8;

/**
 * A group as a folder: the DESIGN.md section 5 background with a mini
 * grid of its first four visible apps (the same rule and layout as the
 * taskbar `.ico`), and the group name under it. Behaves like an AppTile.
 */
export function FolderTile({
  groupId,
  onOpen,
  size = 72,
  shape = "square",
  showLabel = true,
  preview,
}: Props) {
  const group = useGroup(groupId);
  const name = group?.name ?? "";
  const open = () => onOpen?.(groupId);

  return (
    <button
      type="button"
      className="tile focus-ring flex flex-col items-center gap-1 rounded-control"
      style={{ width: size + 2 * INSET, padding: `${INSET}px 0` }}
      title={name}
      aria-label={name}
      data-preview={preview}
      onClick={open}
      {...activationKeys(open)}
    >
      <span className="tile-art block">
        <FolderArt groupId={groupId} size={size} shape={shape} />
      </span>
      {showLabel && (
        <span
          className="type-caption block w-full truncate px-1 text-center"
          data-testid="tile-label"
        >
          {name}
        </span>
      )}
    </button>
  );
}

/**
 * Just the folder picture: the section 5 background and the mini grid of
 * the group's first four visible apps. Decorative; for places such as the
 * Manager sidebar, where the row around it is the control.
 */
export function FolderArt({
  groupId,
  size,
  shape = "square",
}: {
  groupId: string;
  size: number;
  shape?: FolderShape;
}) {
  const apps = useFolderIconApps(groupId);
  return (
    <span
      aria-hidden="true"
      className="relative block shrink-0 bg-folder"
      style={{ width: size, height: size, ...shapeStyle(shape) }}
      data-testid="folder-art"
    >
      {miniLayout(apps.length).map((cell, index) => {
        const appId = apps[index]?.id ?? "";
        return (
          <span
            key={appId}
            className="absolute"
            style={{ left: `${cell.x * 100}%`, top: `${cell.y * 100}%` }}
          >
            <AppIcon
              appId={appId}
              size={Math.round(cell.size * size)}
              radius="var(--radius-mini-icon)"
            />
          </span>
        );
      })}
    </span>
  );
}

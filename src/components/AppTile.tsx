import { useApp } from "../store/hooks";
import { activationKeys, type PreviewState } from "./activation";
import { AppIcon } from "./AppIcon";

type Props = {
  appId: string;
  onActivate?: (appId: string) => void;
  /** Icon side in px (DESIGN.md section 7); the Launchpad medium is 72. */
  size?: number;
  showLabel?: boolean;
  disabled?: boolean;
  /** Style guide only: shows a state without a pointer or keyboard. */
  preview?: PreviewState;
};

/** Space around the icon inside the tile, in px (Fluent 4px step). */
const INSET = 8;

/**
 * An app icon with its name under it. A button: Enter or Space activates
 * it. Hovering lifts and grows the icon; pressing shrinks it (DESIGN.md
 * section 11). The name is one line, cut with "…", full name in the
 * tooltip. A missing app is dimmed and badged.
 */
export function AppTile({
  appId,
  onActivate,
  size = 72,
  showLabel = true,
  disabled = false,
  preview,
}: Props) {
  const app = useApp(appId);
  const name = app?.name ?? "";
  const missing = app?.missing === true;
  const activate = () => {
    if (!disabled) onActivate?.(appId);
  };
  const tooltip = missing ? `${name} can't be found. It may have been moved or uninstalled.` : name;

  return (
    <button
      type="button"
      className="tile focus-ring flex flex-col items-center gap-1 rounded-control"
      style={{ width: size + 2 * INSET, padding: `${INSET}px 0` }}
      disabled={disabled}
      title={tooltip}
      aria-label={tooltip}
      data-preview={preview}
      onClick={activate}
      {...activationKeys(activate)}
    >
      <span className="tile-art relative block">
        <span className="block" data-missing={missing}>
          <AppIcon appId={appId} size={size} />
        </span>
        {missing && <MissingBadge />}
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

/** The "can't be found" badge on a missing app's icon. */
function MissingBadge() {
  return (
    <span
      aria-hidden="true"
      className="absolute -right-1 -bottom-1 flex size-4 items-center justify-center rounded-full bg-danger type-caption text-surface"
      style={{ fontWeight: 600, lineHeight: "16px" }}
    >
      !
    </span>
  );
}

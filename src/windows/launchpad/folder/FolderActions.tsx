import { useState } from "react";
import { Button } from "../../../components/Button";
import { FluentIcon } from "../../../components/FluentIcon";
import { createShortcut, revealShortcut } from "../../manager/groupActions";
import { PinSteps } from "../../manager/PinSteps";
import { launchAll } from "./launchAll";

type Props = {
  groupId: string;
  name: string;
  hasShortcut: boolean;
  /** Whether the folder has any app to launch. */
  empty: boolean;
  /** Hides the Launchpad (after a full Launch All, or once Explorer opens). */
  hide: () => void;
};

/**
 * The folder's actions. "Launch All" opens every app and hides the
 * Launchpad; if some apps did not open, a toast names them and the
 * Launchpad stays so it can be read. "Pin Group to Windows Taskbar"
 * makes the group's shortcut and then shows the one step Windows leaves
 * to the user (the Manager's PinSteps) inside the panel, with "Show in
 * Explorer": the Launchpad is topmost and hides as soon as Explorer takes
 * focus, so the guide comes first and Explorer opens when the user asks.
 * Once the shortcut exists the button reads "Show taskbar shortcut" and
 * goes straight to the guide.
 */
export function FolderActions({ groupId, name, hasShortcut, empty, hide }: Props) {
  const [busy, setBusy] = useState<"launch" | "pin" | "reveal" | null>(null);
  const [guide, setGuide] = useState(false);

  const run = (what: "launch" | "pin" | "reveal", work: () => Promise<void>) => {
    if (busy !== null) return;
    setBusy(what);
    void work().finally(() => setBusy(null));
  };
  const onLaunch = () =>
    run("launch", async () => {
      if (await launchAll(groupId)) hide();
    });
  const onPin = () =>
    run("pin", async () => {
      if (hasShortcut || (await createShortcut(groupId))) setGuide(true);
    });
  const onReveal = () =>
    run("reveal", async () => {
      if (await revealShortcut(groupId)) hide();
    });

  return (
    <div className="mt-4 shrink-0">
      <div className="flex flex-wrap justify-end gap-2">
        {/* One accent button at a time (Fluent): the guide's Show in Explorer. */}
        <Button
          variant={guide ? "secondary" : "primary"}
          onClick={onLaunch}
          disabled={empty || busy !== null}
        >
          {busy === "launch" ? "Launching…" : "Launch All"}
        </Button>
        {!guide && (
          <Button onClick={onPin} disabled={busy !== null}>
            {busy === "pin"
              ? "Creating shortcut…"
              : hasShortcut
                ? "Show taskbar shortcut"
                : "Pin Group to Windows Taskbar"}
          </Button>
        )}
      </div>
      {guide && (
        <div className="max-w-[480px]" data-testid="pin-guide">
          <PinSteps name={name} />
          <div className="mt-4 flex justify-end gap-2">
            <Button variant="primary" onClick={onReveal} disabled={busy !== null}>
              <FluentIcon glyph="folderOpen" />
              Show in Explorer
            </Button>
            <Button onClick={() => setGuide(false)}>Done</Button>
          </div>
        </div>
      )}
    </div>
  );
}

import { useEffect, useRef, useState } from "react";
import { Button } from "../../../components/Button";
import { FolderArt } from "../../../components/FolderTile";
import { FluentIcon } from "../../../components/FluentIcon";
import { api } from "../../../lib/api";
import { onEvent } from "../../../lib/events";
import { useAppStore } from "../../../store/appStore";
import { useGroup } from "../../../store/hooks";
import { groupApps } from "../../../store/selectors";
import { useSetting } from "../../../store/settings";
import { showToast } from "../../../store/toastStore";
import { AppPicker } from "../AppPicker";
import { createShortcut } from "../groupActions";
import { checkGroupName } from "../groupName";
import { KeyChips } from "../settings/HotkeyRecorder";
import { hotkeyParts } from "../settings/hotkeyCombo";
import { appsFound, rescanCount } from "../settings/settingsActions";
import { TaskbarSection } from "../TaskbarSection";

/** How long icons must stop arriving before they count as done (ms). */
const ICONS_QUIET_MS = 1200;

/** Step 1: what Grouper does, and the Launchpad shortcut. */
export function WelcomeStep() {
  const parts = hotkeyParts(useSetting("hotkey"));
  return (
    <div className="flex flex-col gap-4">
      <h1 className="type-title">Welcome to Grouper</h1>
      <p className="type-body">
        Grouper puts folders of apps on your taskbar. Click a folder to open its apps in a small
        panel right above it.
      </p>
      <p className="type-body">
        It also has a Launchpad: all your apps on one full screen, with search and folders.
      </p>
      <div className="flex flex-wrap items-center gap-2 type-body">
        <span>Press</span>
        <KeyChips parts={parts} label={parts.join(" + ")} />
        <span>anywhere to open the Launchpad.</span>
      </div>
      <p className="type-body text-secondary">
        Let's find your apps and make your first folder. It takes about a minute.
      </p>
    </div>
  );
}

/**
 * Step 2: scans the Start Menu once (rescan_apps), then shows the count
 * while the app icons are still being read in the background.
 */
export function ScanStep({
  found,
  onFound,
}: {
  found: number | null;
  onFound: (n: number) => void;
}) {
  const started = useRef(false);
  const [failed, setFailed] = useState(false);
  const [iconsBusy, setIconsBusy] = useState(true);

  useEffect(() => {
    if (started.current || found !== null) return;
    started.current = true;
    void rescanCount().then((count) => {
      if (count === null) setFailed(true);
      else onFound(count);
    });
  }, [found, onFound]);

  useEffect(() => {
    let timer = window.setTimeout(() => setIconsBusy(false), ICONS_QUIET_MS);
    const stop = onEvent("icons:updated", () => {
      setIconsBusy(true);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setIconsBusy(false), ICONS_QUIET_MS);
    });
    return () => {
      stop();
      window.clearTimeout(timer);
    };
  }, []);

  const done = found !== null;
  const status = failed
    ? "Couldn't scan the Start Menu. You can try again later in Settings."
    : !done
      ? "Looking for apps in your Start Menu…"
      : iconsBusy
        ? `Found ${appsFound(found)}. Getting their icons…`
        : `Found ${appsFound(found)}.`;
  return (
    <div className="flex flex-col gap-4">
      <h1 className="type-title">Scanning your apps</h1>
      <p className="type-body text-secondary">
        Grouper reads your Start Menu so every app is ready to put in a folder.
      </p>
      <div
        className="progress-bar"
        role="progressbar"
        aria-label="Scanning your apps"
        data-done={done || failed}
        aria-valuenow={done ? 100 : undefined}
      />
      <p className="type-body-strong" aria-live="polite">
        {status}
      </p>
    </div>
  );
}

type GroupProps = {
  groupId: string | null;
  onGroup: (id: string) => void;
  onShortcut: () => void;
};

/** Step 3: name the group, pick its apps, make its taskbar shortcut. */
export function GroupStep({ groupId, onGroup, onShortcut }: GroupProps) {
  const group = useGroup(groupId ?? "");
  const state = useAppStore((store) => store.state);
  const renameGroup = useAppStore((store) => store.renameGroup);
  const [name, setName] = useState(group?.name ?? "");
  const [error, setError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const [creating, setCreating] = useState(false);
  const count = groupId !== null && state ? groupApps(state, groupId).length : 0;

  const chooseApps = async () => {
    const checked = checkGroupName(name);
    if (!checked.ok) {
      setError(checked.message);
      return;
    }
    setError(null);
    if (groupId === null) {
      const result = await api.createGroup(checked.name);
      if (!result.ok || result.value.id === undefined) {
        console.error(`onboarding: create group: ${result.ok ? "no id" : result.error}`);
        showToast({
          kind: "error",
          title: "Couldn't create the group.",
          message: "Please try again.",
        });
        return;
      }
      onGroup(result.value.id);
    } else if (checked.name !== group?.name) {
      await renameGroup(groupId, checked.name);
    }
    setPicking(true);
  };

  const create = () => {
    if (groupId === null) return;
    setCreating(true);
    void createShortcut(groupId).then((ok) => {
      setCreating(false);
      if (ok) onShortcut();
    });
  };

  return (
    <div className="flex flex-col gap-4">
      <h1 className="type-title">Make your first group</h1>
      <p className="type-body text-secondary">
        A group is a folder of apps you use together, like Work or Games.
      </p>
      <label className="flex flex-col gap-1 type-body">
        Group name
        <input
          className="text-box h-8 w-[320px] max-w-full rounded-control px-2 type-body text-primary"
          value={name}
          placeholder="For example, Work"
          aria-invalid={error !== null}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      {error && <p className="type-caption text-danger">{error}</p>}
      <div className="flex flex-wrap items-center gap-4">
        {groupId !== null && <FolderArt groupId={groupId} size={56} />}
        <div className="flex flex-col gap-2">
          {groupId !== null && <p className="type-body">{appsFound(count)} in this group</p>}
          <div className="flex gap-2">
            <Button onClick={() => void chooseApps()}>
              <FluentIcon glyph="add" />
              {count > 0 ? "Change apps" : "Choose apps"}
            </Button>
            <Button variant="primary" onClick={create} disabled={count === 0 || creating}>
              {creating ? "Creating shortcut…" : "Create taskbar shortcut"}
            </Button>
          </div>
        </div>
      </div>
      {picking && groupId !== null && (
        <AppPicker groupId={groupId} groupName={name.trim()} onClose={() => setPicking(false)} />
      )}
    </div>
  );
}

/** Step 4: show the shortcut in Explorer and how to pin it. */
export function PinStep({ groupId }: { groupId: string | null }) {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="type-title">One last step</h1>
      {groupId === null ? (
        <p className="type-body text-secondary">
          You can make a group any time: click New group in the Manager, then Create taskbar
          shortcut.
        </p>
      ) : (
        <TaskbarSection groupId={groupId} heading={false} />
      )}
    </div>
  );
}

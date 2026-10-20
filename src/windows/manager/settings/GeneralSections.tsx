import { useEffect, useState } from "react";
import { api } from "../../../lib/api";
import { Button } from "../../../components/Button";
import { FluentIcon } from "../../../components/FluentIcon";
import type { FullSettings } from "../../../store/settings";
import { appsFound, openPlace, rescanCount, saveAutostart, saveSettings } from "./settingsActions";
import { SettingCard, SettingSection, Toggle } from "./SettingCard";

/** Start with Windows, Rescan Start Menu and Show welcome again. */
export function StartupSection({ settings }: { settings: FullSettings }) {
  const [scanning, setScanning] = useState(false);
  const [found, setFound] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);

  const rescan = () => {
    setScanning(true);
    void rescanCount()
      .then(setFound)
      .finally(() => setScanning(false));
  };

  const autostart = (on: boolean) => {
    setSaving(true);
    void saveAutostart(on).finally(() => setSaving(false));
  };

  return (
    <SettingSection title="Startup and apps">
      <SettingCard
        title="Start with Windows"
        description="Grouper starts in the tray when you sign in, without opening a window."
      >
        <Toggle
          label="Start with Windows"
          checked={settings.autostart}
          disabled={saving}
          onChange={autostart}
        />
      </SettingCard>
      <SettingCard
        title="Rescan Start Menu"
        description={
          <span aria-live="polite">
            {scanning
              ? "Scanning the Start Menu…"
              : found === null
                ? "Looks for apps you installed since the last scan."
                : `Found ${appsFound(found)}.`}
          </span>
        }
      >
        <Button onClick={rescan} disabled={scanning}>
          {scanning ? (
            <span aria-hidden="true" className="progress-ring" />
          ) : (
            <FluentIcon glyph="refresh" />
          )}
          {scanning ? "Scanning…" : "Rescan now"}
        </Button>
      </SettingCard>
      <SettingCard
        title="Show welcome again"
        description="Walks you through making a group and pinning it."
      >
        <Button onClick={() => void saveSettings({ firstRunDone: false })}>Show welcome</Button>
      </SettingCard>
    </SettingSection>
  );
}

/** Open logs folder and Open data folder. */
export function TroubleshootingSection() {
  return (
    <SettingSection title="Troubleshooting">
      <SettingCard title="Logs" description="What Grouper did, for reporting a problem.">
        <Button onClick={() => void openPlace("logs")}>
          <FluentIcon glyph="folderOpen" />
          Open logs folder
        </Button>
      </SettingCard>
      <SettingCard title="Data" description="Your groups, settings and cached icons.">
        <Button onClick={() => void openPlace("data")}>
          <FluentIcon glyph="folderOpen" />
          Open data folder
        </Button>
      </SettingCard>
    </SettingSection>
  );
}

/** About: the version and a link to the repo. */
export function AboutSection() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    void api.appVersion().then((result) => {
      if (result.ok) setVersion(result.value);
      else console.error(`settings: version: ${result.error}`);
    });
  }, []);
  return (
    <SettingSection title="About">
      <SettingCard
        title="Grouper"
        description={
          version === null ? "Taskbar folders and a Launchpad for Windows." : `Version ${version}`
        }
      >
        <Button variant="ghost" onClick={() => void openPlace("repo")}>
          View on GitHub
        </Button>
      </SettingCard>
    </SettingSection>
  );
}

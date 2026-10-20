import { displayHotkey } from "../../../lib/hotkeyToast";
import { useHotkeyStatus } from "../../../store/hotkeyStatus";
import { useSettings, type FullSettings } from "../../../store/settings";
import { AppearanceSection } from "./AppearanceSection";
import { AboutSection, StartupSection, TroubleshootingSection } from "./GeneralSections";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { saveSettings } from "./settingsActions";
import { SettingCard, SettingSection, Toggle } from "./SettingCard";

/**
 * The Fluent warning InfoBar shown while the Launchpad shortcut could not
 * be registered at startup (DESIGN.md section 13 wording).
 */
function HotkeyBanner() {
  const failed = useHotkeyStatus((status) => status.failed);
  if (failed === null) return null;
  return (
    <div
      role="alert"
      className="info-bar mb-6 flex items-start gap-3 rounded-control border border-stroke-card px-4 py-3"
    >
      <span
        aria-hidden="true"
        className="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full bg-caution type-caption font-semibold text-surface"
      >
        !
      </span>
      <p className="type-body">
        <span className="type-body-strong">
          {displayHotkey(failed.hotkey)} is already used by another app.
        </span>{" "}
        Pick a different shortcut in Settings.
      </p>
    </div>
  );
}

/** The Launchpad shortcut and the hot corners. */
function LaunchpadSection({ settings }: { settings: FullSettings }) {
  const corners = settings.hotCorners;
  return (
    <SettingSection title="Launchpad">
      <HotkeyRecorder hotkey={settings.hotkey} />
      <SettingCard
        title="Hot corner: top left"
        description="Move the mouse into the top-left corner of the screen to open the Launchpad."
      >
        <Toggle
          label="Hot corner: top left"
          checked={corners.topLeft !== false}
          onChange={(topLeft) => void saveSettings({ hotCorners: { ...corners, topLeft } })}
        />
      </SettingCard>
      <SettingCard
        title="Hot corner: top right"
        description="Move the mouse into the top-right corner of the screen to open the Launchpad."
      >
        <Toggle
          label="Hot corner: top right"
          checked={corners.topRight !== false}
          onChange={(topRight) => void saveSettings({ hotCorners: { ...corners, topRight } })}
        />
      </SettingCard>
    </SettingSection>
  );
}

/**
 * The Manager's Settings page (DESIGN.md section 9), laid out like
 * Windows Settings: sections of setting cards, each with its title and
 * description on the left and its control on the right. Light and dark
 * follow Windows, so there is no theme setting.
 */
export function SettingsPage() {
  const settings = useSettings();
  return (
    <section aria-label="Settings" className="flex max-w-[1000px] flex-col">
      <h1 className="mb-6 py-1 type-title">Settings</h1>
      <HotkeyBanner />
      <LaunchpadSection settings={settings} />
      <AppearanceSection settings={settings} />
      <StartupSection settings={settings} />
      <TroubleshootingSection />
      <AboutSection />
    </section>
  );
}

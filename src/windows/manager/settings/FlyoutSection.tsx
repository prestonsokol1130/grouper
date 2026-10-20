import type { FlyoutGrid, FlyoutIconSize } from "../../../bindings";
import type { FullSettings } from "../../../store/settings";
import { saveSettings } from "./settingsActions";
import { Select, SettingCard, SettingSection, Toggle } from "./SettingCard";

/** "Flyout layout": apps across x down on each flyout page. */
const LAYOUTS = [
  ["3x3", "3 × 3 (recommended)"],
  ["4x4", "4 × 4"],
  ["5x5", "5 × 5"],
  ["4x2", "4 × 2"],
  ["3x2", "3 × 2"],
  ["2x2", "2 × 2"],
] as const satisfies readonly (readonly [FlyoutGrid, string])[];

/** "Flyout icon size": follow the taskbar, or a fixed size. */
const FLYOUT_SIZES = [
  ["matchTaskbar", "Match taskbar (recommended)"],
  ["small", "Small"],
  ["medium", "Medium"],
  ["large", "Large"],
] as const satisfies readonly (readonly [FlyoutIconSize, string])[];

/**
 * The taskbar flyout's settings (DESIGN.md section 9, added per
 * Preston's feedback, 2026-10-20): its page layout, its icon size and
 * the optional page arrows. Each applies the next time a flyout opens.
 */
export function FlyoutSection({ settings }: { settings: FullSettings }) {
  return (
    <SettingSection title="Taskbar flyout">
      <SettingCard
        title="Flyout layout"
        description="Apps across and down on each page. More apps go on more pages."
      >
        <Select
          label="Flyout layout"
          value={settings.flyoutLayout}
          options={LAYOUTS}
          onChange={(flyoutLayout) => void saveSettings({ flyoutLayout })}
        />
      </SettingCard>
      <SettingCard
        title="Flyout icon size"
        description="Match taskbar follows your taskbar's icon size and display scaling."
      >
        <Select
          label="Flyout icon size"
          value={settings.flyoutIconSize}
          options={FLYOUT_SIZES}
          onChange={(flyoutIconSize) => void saveSettings({ flyoutIconSize })}
        />
      </SettingCard>
      <SettingCard
        title="Show page arrows"
        description="Adds ‹ › buttons beside the page dots in the flyout and the Launchpad."
      >
        <Toggle
          label="Show page arrows"
          checked={settings.showPageArrows}
          onChange={(showPageArrows) => void saveSettings({ showPageArrows })}
        />
      </SettingCard>
    </SettingSection>
  );
}

/** Accessibility: "Always show keyboard focus". */
export function AccessibilitySection({ settings }: { settings: FullSettings }) {
  return (
    <SettingSection title="Accessibility">
      <SettingCard
        title="Always show keyboard focus"
        description="Shows the focus ring as soon as the flyout or Launchpad opens, and on every focused control."
      >
        <Toggle
          label="Always show keyboard focus"
          checked={settings.alwaysShowFocus}
          onChange={(alwaysShowFocus) => void saveSettings({ alwaysShowFocus })}
        />
      </SettingCard>
    </SettingSection>
  );
}

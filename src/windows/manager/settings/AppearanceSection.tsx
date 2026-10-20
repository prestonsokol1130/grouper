import type { AnimationStyle, BackgroundSetting, FolderShape, IconSize } from "../../../bindings";
import { FALLBACK_ACCENT } from "../../../lib/theme";
import { GRID_LIMITS, type FullSettings } from "../../../store/settings";
import { saveSettings } from "./settingsActions";
import { Select, SettingCard, SettingSection } from "./SettingCard";

const BACKGROUNDS = [
  ["mixed", "Mixed (recommended)"],
  ["mica", "Mica"],
  ["acrylic", "Acrylic"],
  ["solid", "Solid"],
] as const satisfies readonly (readonly [BackgroundSetting, string])[];

const SHAPES = [
  ["square", "Windows square"],
  ["squircle", "iPhone squircle"],
] as const satisfies readonly (readonly [FolderShape, string])[];

const SIZES = [
  ["small", "Small"],
  ["medium", "Medium"],
  ["large", "Large"],
] as const satisfies readonly (readonly [IconSize, string])[];

const ANIMATIONS = [
  ["genie", "Genie"],
  ["scale", "Scale"],
  ["smooth", "Windows smooth"],
] as const satisfies readonly (readonly [AnimationStyle, string])[];

/** "4" to "10", for a grid select. */
function range({ min, max }: { min: number; max: number }): (readonly [string, string])[] {
  return Array.from({ length: max - min + 1 }, (_, i) => {
    const n = String(min + i);
    return [n, n] as const;
  });
}

/** The "Accent color" card: Windows accent or a custom colour. */
function AccentCard({ accent }: { accent: string }) {
  const custom = accent !== "windows";
  return (
    <SettingCard
      title="Accent color"
      description="Used for buttons, selection and the Launchpad page dots."
    >
      <Select
        label="Accent color"
        value={custom ? "custom" : "windows"}
        options={[
          ["windows", "Use Windows accent"],
          ["custom", "Custom color"],
        ]}
        onChange={(choice) =>
          void saveSettings({ accent: choice === "windows" ? "windows" : FALLBACK_ACCENT.light })
        }
      />
      {custom && (
        <input
          type="color"
          aria-label="Custom accent color"
          className="color-well focus-ring"
          value={accent.toLowerCase()}
          onChange={(event) => void saveSettings({ accent: event.target.value.toUpperCase() })}
        />
      )}
    </SettingCard>
  );
}

/**
 * The look-and-feel settings (DESIGN.md section 9). Each applies live in
 * every window: Rust pushes the new settings with `state:changed`.
 */
export function AppearanceSection({ settings }: { settings: FullSettings }) {
  return (
    <SettingSection title="Appearance">
      <AccentCard accent={settings.accent} />
      <SettingCard
        title="Window background"
        description="Mixed uses Mica for the Manager and Acrylic for the flyout and Launchpad."
      >
        <Select
          label="Window background"
          value={settings.background}
          options={BACKGROUNDS}
          onChange={(background) => void saveSettings({ background })}
        />
      </SettingCard>
      <SettingCard
        title="Folder shape"
        description="For folders in the Launchpad and your taskbar icons."
      >
        <Select
          label="Folder shape"
          value={settings.folderShape}
          options={SHAPES}
          onChange={(folderShape) => void saveSettings({ folderShape })}
        />
      </SettingCard>
      <SettingCard
        title="Launchpad icon size"
        description="App icons in the Launchpad. The flyout has its own size below."
      >
        <Select
          label="Launchpad icon size"
          value={settings.iconSize}
          options={SIZES}
          onChange={(iconSize) => void saveSettings({ iconSize })}
        />
      </SettingCard>
      <SettingCard title="Launchpad grid" description="Apps across and down on each page.">
        <Select
          label="Columns"
          value={String(settings.gridColumns)}
          options={range(GRID_LIMITS.columns)}
          onChange={(value) => void saveSettings({ gridColumns: Number(value) })}
        />
        <span aria-hidden="true" className="type-body text-secondary">
          ×
        </span>
        <Select
          label="Rows"
          value={String(settings.gridRows)}
          options={range(GRID_LIMITS.rows)}
          onChange={(value) => void saveSettings({ gridRows: Number(value) })}
        />
      </SettingCard>
      <SettingCard
        title="Animation style"
        description="How the flyout and the Launchpad open and close."
      >
        <Select
          label="Animation style"
          value={settings.animation}
          options={ANIMATIONS}
          onChange={(animation) => void saveSettings({ animation })}
        />
      </SettingCard>
    </SettingSection>
  );
}

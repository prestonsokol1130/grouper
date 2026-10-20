import { Button } from "../../components/Button";
import { FluentIcon } from "./FluentIcon";

/** Shown in the content pane when there are no groups (DESIGN.md section 13 voice). */
export function NoGroups({ onNewGroup }: { onNewGroup: () => void }) {
  return (
    <section className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <h1 className="type-subtitle">No groups yet.</h1>
      <p className="mb-4 type-body text-secondary">Click New group to make your first one.</p>
      <Button variant="primary" onClick={onNewGroup}>
        <FluentIcon glyph="add" />
        New group
      </Button>
    </section>
  );
}

/** A stand-in until the Settings page arrives (S6-01). */
export function SettingsPlaceholder() {
  return (
    <section aria-label="Settings" className="flex flex-col">
      <h1 className="mb-2 py-1 type-title">Settings</h1>
      <p className="type-body text-secondary">
        Settings come later. For now, Grouper follows your Windows theme and accent color.
      </p>
    </section>
  );
}

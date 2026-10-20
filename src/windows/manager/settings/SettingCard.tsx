import { useId, type ReactNode } from "react";

/**
 * A group of setting cards under a Body Strong heading, like the sections
 * of a Windows Settings page.
 */
export function SettingSection({ title, children }: { title: string; children: ReactNode }) {
  const id = useId();
  return (
    <section aria-labelledby={id} className="mb-6 flex flex-col gap-1">
      <h2 id={id} className="mb-1 type-body-strong">
        {title}
      </h2>
      {children}
    </section>
  );
}

type CardProps = {
  title: string;
  description?: ReactNode;
  /** The control, shown on the right. */
  children?: ReactNode;
  /** Shown under the row, full width (for example an error). */
  footer?: ReactNode;
  /** Gives the control a label: the id of the title. */
  titleId?: string;
};

/**
 * A Fluent settings card (WinUI SettingsCard): title and description on
 * the left, the control on the right, 4px corners on the card fill.
 */
export function SettingCard({ title, description, children, footer, titleId }: CardProps) {
  return (
    <div className="setting-card rounded-control border border-stroke-card bg-card px-4 py-3">
      <div className="flex min-h-10 flex-wrap items-center gap-x-6 gap-y-2">
        <div className="min-w-[200px] flex-1">
          <p id={titleId} className="type-body">
            {title}
          </p>
          {description && <div className="type-caption text-secondary">{description}</div>}
        </div>
        {children !== undefined && (
          <div className="flex shrink-0 flex-wrap items-center gap-2">{children}</div>
        )}
      </div>
      {footer}
    </div>
  );
}

type ToggleProps = {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
};

/** A Fluent ToggleSwitch: "On" or "Off" to the left of a 40 x 20 pill. */
export function Toggle({ label, checked, disabled, onChange }: ToggleProps) {
  return (
    <label className="flex cursor-pointer items-center gap-3 type-body select-none">
      <span className="w-6 text-right">{checked ? "On" : "Off"}</span>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        className="toggle-switch focus-ring"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

type SelectProps<T extends string> = {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (value: T) => void;
};

/** A Fluent ComboBox: a native select drawn like a TextBox. */
export function Select<T extends string>({ label, value, options, onChange }: SelectProps<T>) {
  return (
    <select
      aria-label={label}
      className="text-box h-8 min-w-[180px] rounded-control px-2 type-body text-primary"
      value={value}
      onChange={(event) => {
        const next = options.find(([option]) => option === event.target.value);
        if (next) onChange(next[0]);
      }}
    >
      {options.map(([option, text]) => (
        <option key={option} value={option}>
          {text}
        </option>
      ))}
    </select>
  );
}

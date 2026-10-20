import type { ReactNode } from "react";

/** A titled block of the style guide. */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-10">
      <h2 className="type-subtitle mb-4">{title}</h2>
      {children}
    </section>
  );
}

const COLORS = [
  "--text-primary",
  "--text-secondary",
  "--text-disabled",
  "--fill-subtle-hover",
  "--fill-subtle-pressed",
  "--card-bg",
  "--stroke-card",
  "--stroke-surface",
  "--solid-bg",
  "--layer-bg",
  "--danger",
  "--success",
  "--accent",
  "--accent-hover",
  "--accent-pressed",
  "--text-on-accent",
  "--accent-disabled",
  "--focus-outer",
  "--focus-inner",
  "--folder-bg",
  "--folder-ico-bg",
  "--folder-ico-border",
];

const RADII = [
  ["--radius-control", "controls"],
  ["--radius-window", "windows, flyout, cards"],
  ["--radius-tile", "folder tile (18%)"],
  ["--radius-mini-icon", "mini icons (22%)"],
  ["--radius-search", "Launchpad search pill"],
];

/** The value of custom property `name` on `<html>` right now. */
function current(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** Colour swatches, radii, the raised shadow and material tints. */
export function TokensSection() {
  return (
    <>
      <Section title="Colour">
        <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3">
          {COLORS.map((name) => (
            <div key={name} className="flex items-center gap-3">
              {/* A checkerboard shows translucent tokens honestly. */}
              <span
                className="size-10 shrink-0 rounded-control border border-stroke-surface"
                style={{
                  background: `linear-gradient(var(${name}), var(${name})), repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 0 0 / 10px 10px`,
                }}
              />
              <span className="min-w-0">
                <code className="type-caption block">{name}</code>
                <span className="type-caption block truncate text-secondary">{current(name)}</span>
              </span>
            </div>
          ))}
        </div>
      </Section>
      <Section title="Shape and elevation">
        <div className="flex flex-wrap gap-6">
          {RADII.map(([name, use]) => (
            <div key={name} className="flex w-36 flex-col items-center gap-2 text-center">
              <span
                className="size-16 border border-stroke-surface bg-card"
                style={{ borderRadius: `var(${name})` }}
              />
              <code className="type-caption">{name}</code>
              <span className="type-caption text-secondary">{use}</span>
            </div>
          ))}
          <div className="flex w-36 flex-col items-center gap-2 text-center">
            <span className="size-16 rounded-card bg-layer shadow-raised" />
            <code className="type-caption">--shadow-raised</code>
            <span className="type-caption text-secondary">hover lift shadow</span>
          </div>
        </div>
      </Section>
      <Section title="Materials">
        <p className="type-body mb-2 text-secondary">
          Manager: Mica. Flyout: Acrylic, tint {current("--acrylic-tint-flyout")}. Launchpad:
          Acrylic, tint {current("--acrylic-tint-launchpad")}. Solid ({"--solid-bg"}) when
          transparency effects are off, on battery saver or when unsupported. This page:{" "}
          <strong>{document.documentElement.dataset.material ?? "unknown"}</strong>.
        </p>
      </Section>
    </>
  );
}

/**
 * How to pin the group's shortcut, in the DESIGN.md section 13 voice.
 * Windows doesn't let apps pin themselves, so this one step is the
 * user's. A small menu drawn with the theme tokens shows what to click.
 */
export function PinSteps({ name }: { name: string }) {
  const steps = [
    "Click Show in Explorer.",
    `Right-click ${name || "the shortcut"}.`,
    "If you don't see Pin to taskbar, choose Show more options.",
    "Choose Pin to taskbar.",
  ];
  return (
    <div className="mt-5">
      <h3 className="type-body-strong">Pin it to your taskbar</h3>
      <p className="mb-3 type-body text-secondary">
        Windows doesn't let apps pin themselves, so this one step is up to you.
      </p>
      <div className="flex flex-wrap items-start gap-6">
        <ol className="flex min-w-[240px] flex-1 flex-col gap-2">
          {steps.map((step, index) => (
            <li key={step} className="flex items-start gap-3 type-body">
              <span
                aria-hidden="true"
                className="flex size-5 shrink-0 items-center justify-center rounded-full bg-accent type-caption font-semibold text-on-accent"
              >
                {index + 1}
              </span>
              <span>{step}</span>
            </li>
          ))}
        </ol>
        <MenuSketch />
      </div>
    </div>
  );
}

/** A decorative right-click menu with Pin to taskbar highlighted. */
function MenuSketch() {
  const rows = ["Open", "Pin to Start", "Pin to taskbar", "Show more options"];
  return (
    <div
      aria-hidden="true"
      className="w-[200px] shrink-0 rounded-window border border-stroke-card bg-layer p-1 shadow-raised"
    >
      {rows.map((row) => {
        const pin = row === "Pin to taskbar";
        return (
          <div
            key={row}
            className={`relative rounded-control px-3 py-1.5 type-body ${pin ? "bg-subtle-hover" : "text-secondary"}`}
          >
            {pin && (
              <span className="absolute top-1/2 left-0 h-4 w-[3px] -translate-y-1/2 rounded-full bg-accent" />
            )}
            {row}
          </div>
        );
      })}
    </div>
  );
}

import { useRef } from "react";
import { Button } from "../../components/Button";
import {
  DEFAULT_OPEN_CLOSE,
  DROP_TARGET,
  HOVER_LIFT,
  OPEN_CLOSE,
  PAGE_SWIPE,
  PRESS,
  REDUCED_FADE,
  openCloseTiming,
  usePrefersReducedMotion,
  type Timing,
} from "../../lib/motion";
import { Section } from "./TokensSection";

const RAMP = [
  ["type-caption", "Caption 12/16 Regular", "Text"],
  ["type-body", "Body 14/20 Regular", "Text"],
  ["type-body-strong", "Body Strong 14/20 Semibold", "Text"],
  ["type-subtitle", "Subtitle 20/28 Semibold", "Display"],
  ["type-title", "Title 28/36 Semibold", "Display"],
  ["type-title-large", "Title Large 40/52 Semibold", "Display"],
];

/** The Fluent type ramp in Segoe UI Variable. */
export function TypeSection() {
  return (
    <Section title="Typography">
      <div className="flex flex-col gap-3">
        {RAMP.map(([className, label, optical]) => (
          <div key={className} className="flex items-baseline gap-4">
            <code className="type-caption w-40 shrink-0 text-secondary">.{className}</code>
            <span className={className}>{label}</span>
            <span className="type-caption text-secondary">Segoe UI Variable {optical}</span>
          </div>
        ))}
      </div>
    </Section>
  );
}

/** A dot that crosses its track with `timing` when played. */
function TimingDemo({ name, timing, note }: { name: string; timing: Timing; note: string }) {
  const dot = useRef<HTMLSpanElement>(null);
  const play = () => {
    dot.current?.animate([{ transform: "translateX(0)" }, { transform: "translateX(200px)" }], {
      duration: timing.durationMs,
      easing: timing.easing,
      fill: "forwards",
    });
  };
  return (
    <div className="flex items-center gap-4">
      <Button variant="ghost" onClick={play}>
        Play
      </Button>
      <span className="relative block h-4 w-[216px] rounded-full bg-subtle-hover">
        <span ref={dot} className="absolute top-0 left-0 block size-4 rounded-full bg-accent" />
      </span>
      <span className="type-body-strong w-36">{name}</span>
      <code className="type-caption text-secondary">
        {timing.durationMs}ms {timing.easing}
      </code>
      <span className="type-caption text-secondary">{note}</span>
    </div>
  );
}

/** Every motion preset from `src/lib/motion.ts`. */
export function MotionSection() {
  const reduced = usePrefersReducedMotion();
  return (
    <Section title="Motion">
      <p className="type-body mb-3 text-secondary">
        Reduced motion is {reduced ? "on" : "off"}: open/close uses{" "}
        {reduced ? "the 120ms fade" : "each style's own timing"}.
      </p>
      <div className="flex flex-col gap-2">
        {Object.entries(OPEN_CLOSE).map(([key, preset]) => (
          <TimingDemo
            key={key}
            name={key === DEFAULT_OPEN_CLOSE ? `${preset.label} (default)` : preset.label}
            timing={openCloseTiming(key as keyof typeof OPEN_CLOSE, reduced)}
            note={`Open: ${preset.open}. Close: ${preset.close}.`}
          />
        ))}
        <TimingDemo name="Reduced fade" timing={REDUCED_FADE} note="Animation effects off" />
        <TimingDemo
          name="Hover lift"
          timing={HOVER_LIFT}
          note={`Rise ${HOVER_LIFT.liftPx}px, scale ${HOVER_LIFT.scale}, shadow ${HOVER_LIFT.shadow}`}
        />
        <TimingDemo name="Pressed" timing={PRESS} note={`Scale ${PRESS.scale}`} />
        <TimingDemo name="Page swipe" timing={PAGE_SWIPE} note="Launchpad pages" />
      </div>
      <p className="type-caption mt-3 text-secondary">
        Drop target scale while dragging: {DROP_TARGET.scale}. The Genie shape itself comes with the
        window animation tickets; this page shows its timing.
      </p>
    </Section>
  );
}

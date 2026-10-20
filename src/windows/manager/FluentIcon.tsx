/** Segoe Fluent Icons code points (the same in Segoe MDL2 Assets). */
const GLYPHS = {
  add: "",
  settings: "",
} as const;

/** A decorative 16px Windows system icon. */
export function FluentIcon({ glyph }: { glyph: keyof typeof GLYPHS }) {
  return (
    <span aria-hidden="true" className="fluent-icon inline-block w-4 shrink-0 text-center">
      {GLYPHS[glyph]}
    </span>
  );
}

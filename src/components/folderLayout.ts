/** Folder tile shapes, DESIGN.md section 5. Windows square is the default. */
export type FolderShape = "square" | "squircle";

/** A mini icon's place, as fractions of the tile side. */
export type MiniCell = { x: number; y: number; size: number };

/** DESIGN.md section 5 recipe, shared with the taskbar `.ico` (Rust `FOLDER_STYLE`). */
const PADDING = 0.14;
const GAP = 0.06;
/** How much bigger than a grid cell a folder's only icon is drawn. */
const SINGLE_ICON_SCALE = 1.6;
/** iPhone squircle exponent. */
const SQUIRCLE_N = 5;

/**
 * Where `count` mini icons go, like `layout` in `taskbar/compose.rs`:
 * 1 is centred and 1.6x a cell, 2 sit side by side centred vertically,
 * 3 are two on top and one centred below, 4 fill the 2x2 grid.
 */
export function miniLayout(count: number): MiniCell[] {
  const cell = (1 - 2 * PADDING - GAP) / 2;
  const near = PADDING;
  const far = PADDING + cell + GAP;
  const centre = (1 - cell) / 2;
  const at = (x: number, y: number): MiniCell => ({ x, y, size: cell });
  switch (Math.min(count, 4)) {
    case 0:
      return [];
    case 1: {
      const big = cell * SINGLE_ICON_SCALE;
      const offset = (1 - big) / 2;
      return [{ x: offset, y: offset, size: big }];
    }
    case 2:
      return [at(near, centre), at(far, centre)];
    case 3:
      return [at(near, near), at(far, near), at(centre, far)];
    default:
      return [at(near, near), at(far, near), at(near, far), at(far, far)];
  }
}

/** A CSS `polygon()` tracing the superellipse |x|^n + |y|^n = 1. */
function squirclePolygon(points = 96): string {
  const exponent = 2 / SQUIRCLE_N;
  const coords: string[] = [];
  for (let i = 0; i < points; i += 1) {
    const t = (i / points) * 2 * Math.PI;
    const cos = Math.cos(t);
    const sin = Math.sin(t);
    const x = Math.sign(cos) * Math.abs(cos) ** exponent;
    const y = Math.sign(sin) * Math.abs(sin) ** exponent;
    coords.push(`${(50 + 50 * x).toFixed(2)}% ${(50 + 50 * y).toFixed(2)}%`);
  }
  return `polygon(${coords.join(", ")})`;
}

const SQUIRCLE = squirclePolygon();

/** The CSS that gives a tile its folder shape. */
export function shapeStyle(shape: FolderShape): { borderRadius?: string; clipPath?: string } {
  return shape === "squircle" ? { clipPath: SQUIRCLE } : { borderRadius: "var(--radius-tile)" };
}
